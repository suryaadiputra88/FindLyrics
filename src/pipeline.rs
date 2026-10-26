use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

use crate::models::{LyricState, ProcessOptions, TrackItem};
use crate::providers::MultiProviderClient;
use crate::tagger::embed_lyrics_to_file;

#[derive(Debug, Default)]
pub struct BatchSummary {
    pub total: usize,
    pub embedded: usize,
    pub skipped: usize,
    pub not_found: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackProcessResult {
    Embedded,
    Skipped,
    NotFound,
    Error,
}

async fn process_single_track(
    track: &TrackItem,
    client: &MultiProviderClient,
    options: &ProcessOptions,
    pb: &ProgressBar,
) -> TrackProcessResult {
    let is_synced = track.lyric_state == LyricState::Synced;
    let is_plain = track.lyric_state == LyricState::Plain;

    // Skip if already has synced lyrics and overwrite is false
    if is_synced && !options.overwrite {
        if options.verbose {
            pb.println(format!(
                "{} {} (sudah memiliki synced lyrics)",
                "⏭ Dilewati:".dimmed(),
                track.filename.dimmed()
            ));
        }
        return TrackProcessResult::Skipped;
    }

    // If has plain lyrics and upgrade is disabled and overwrite is false -> skip
    if is_plain && !options.overwrite && !options.upgrade_plain {
        if options.verbose {
            pb.println(format!(
                "{} {} (sudah memiliki lirik teks)",
                "⏭ Dilewati:".dimmed(),
                track.filename.dimmed()
            ));
        }
        return TrackProcessResult::Skipped;
    }

    // Check if track has title metadata
    if track.title.is_none() {
        pb.println(format!(
            "{} {} (Metadata Title kosong)",
            "⚠️ Gagal:".yellow(),
            track.filename
        ));
        return TrackProcessResult::Error;
    }

    // When upgrading from plain lyrics, we specifically require synced lyrics
    let require_synced = options.synced_only || (is_plain && !options.overwrite);

    // Fetch lyrics (Step 1: Mengunduh)
    if options.verbose {
        pb.println(format!(
            "{} {}",
            "⬇️  Mengunduh:".blue(),
            track.filename
        ));
    }

    match client
        .fetch_lyrics(track, options.tolerance_secs, require_synced)
        .await
    {
        Ok(Some(payload)) => {
            let lyric_type_str = if payload.is_synced {
                "Synced (LRC)".green().bold()
            } else {
                "Plain Text".blue()
            };

            let action_prefix = if is_plain && payload.is_synced {
                "🔄 Upgrade:".cyan().bold()
            } else {
                "✓ Sukses:".green().bold()
            };

            if let Some(best_lyrics) = payload.best_lyrics() {
                if options.dry_run {
                    pb.println(format!(
                        "{} {} -> Ditemukan via {} ({}) [DRY RUN - Tidak Ditanamkan]",
                        "🔍 Match:".cyan(),
                        track.filename.bold(),
                        payload.provider.magenta(),
                        lyric_type_str
                    ));
                    TrackProcessResult::Embedded
                } else {
                    // Step 2: Menanamkan / Menyisipkan ke metadata audio
                    if options.verbose {
                        pb.println(format!(
                            "{} {} (via {})",
                            "💾 Menanamkan:".magenta(),
                            track.filename,
                            payload.provider
                        ));
                    }
                    match embed_lyrics_to_file(&track.path, best_lyrics) {
                        Ok(()) => {
                            pb.println(format!(
                                "{} {} -> Berhasil ditanamkan via {} ({}, diff: {:.1}s)",
                                action_prefix,
                                track.filename.bold(),
                                payload.provider.magenta().bold(),
                                lyric_type_str,
                                payload.duration_diff.abs()
                            ));
                            TrackProcessResult::Embedded
                        }
                        Err(e) => {
                            pb.println(format!(
                                "{} {} -> Gagal menanamkan tag: {}",
                                "✗ Error:".red().bold(),
                                track.filename,
                                e
                            ));
                            TrackProcessResult::Error
                        }
                    }
                }
            } else {
                pb.println(format!(
                    "{} {} -> Konten lirik kosong dari provider",
                    "⚠️ Kosong:".yellow(),
                    track.filename
                ));
                TrackProcessResult::NotFound
            }
        }
        Ok(None) => {
            if options.verbose {
                pb.println(format!(
                    "{} {} -> Lirik tidak ditemukan",
                    "✗ Tidak Ditemukan:".yellow(),
                    track.filename
                ));
            }
            TrackProcessResult::NotFound
        }
        Err(e) => {
            pb.println(format!(
                "{} {} -> Error koneksi/query: {}",
                "✗ Error:".red(),
                track.filename,
                e
            ));
            TrackProcessResult::Error
        }
    }
}

pub async fn run_fetch_and_embed(
    tracks: &[TrackItem],
    options: &ProcessOptions,
) -> Result<BatchSummary> {
    let client = Arc::new(MultiProviderClient::new());
    let concurrency = options.concurrency.max(1);
    let options = Arc::new(options.clone());

    let mut summary = BatchSummary {
        total: tracks.len(),
        ..Default::default()
    };

    println!(
        "\n{} (Multi-thread: {} worker paralel)",
        "🚀 Memulai Proses Fetch & Embed Lyrics...".green().bold(),
        concurrency.to_string().cyan().bold()
    );

    let pb = Arc::new(ProgressBar::new(tracks.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!("{} worker aktif", concurrency));

    let start_time = Instant::now();
    let mut join_set = JoinSet::new();

    let record_result = |summary: &mut BatchSummary, res: TrackProcessResult| {
        match res {
            TrackProcessResult::Embedded => summary.embedded += 1,
            TrackProcessResult::Skipped => summary.skipped += 1,
            TrackProcessResult::NotFound => summary.not_found += 1,
            TrackProcessResult::Error => summary.errors += 1,
        }
    };

    for track in tracks.iter().cloned() {
        while join_set.len() >= concurrency {
            if let Some(res) = join_set.join_next().await {
                match res {
                    Ok(item_res) => record_result(&mut summary, item_res),
                    Err(_) => summary.errors += 1,
                }
            }
        }

        let client = Arc::clone(&client);
        let options = Arc::clone(&options);
        let pb = Arc::clone(&pb);

        join_set.spawn(async move {
            let res = process_single_track(&track, &client, &options, &pb).await;
            pb.inc(1);
            res
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(item_res) => record_result(&mut summary, item_res),
            Err(_) => summary.errors += 1,
        }
    }

    pb.finish_with_message("Selesai memproses semua file!");

    let elapsed = start_time.elapsed();
    println!("\n{}", "=========================================================================================".cyan().bold());
    println!("{}", "                              RINGKASAN PROSES BATCH                                    ".cyan().bold());
    println!("{}\n", "=========================================================================================".cyan().bold());
    println!("⏱  Waktu Proses          : {:.2?}", elapsed);
    println!("📊 Total File             : {}", summary.total);
    println!("✅ Berhasil Ditanamkan    : {}", summary.embedded.to_string().green().bold());
    println!("⏭  Dilewati (Sudah ada)  : {}", summary.skipped.to_string().cyan());
    println!("❓ Tidak Ditemukan        : {}", summary.not_found.to_string().yellow());
    println!("❌ Gagal / Error          : {}", summary.errors.to_string().red());
    println!("⚡ Kapasitas Paralel      : {} thread/worker", concurrency);
    println!("{}\n", "=========================================================================================".cyan());

    Ok(summary)
}
