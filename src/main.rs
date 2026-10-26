use std::path::PathBuf;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use findlyrics::models::ProcessOptions;
use findlyrics::pipeline::run_fetch_and_embed;
use findlyrics::scanner::scan_directory_with_progress;
use findlyrics::ui::{print_track_list, prompt_user_action, UserAction};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(
    name = "findlyrics",
    author = "bsa",
    version = "0.1.0",
    about = "🎵 Pencari Lirik Musik & Direct Audio Tag Embedder (FLAC, WAV, MP3, M4A, OGG)",
    long_about = "FindLyrics memindai file musik dalam satu direktori, menampilkan daftar status lirik, \
lalu mengunduh dan menyematkan (embed) lirik langsung ke dalam metadata file audio tanpa membuat file .lrc terpisah."
)]
struct Cli {
    /// Path ke direktori musik atau file audio tunggal
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Pindai subdirektori secara rekursif
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Timpa (overwrite) lirik pada file yang sudah memiliki lirik
    #[arg(short = 'o', long)]
    overwrite: bool,

    /// Hanya simpan jika menemukan lirik berwaktu / time-synced (LRC)
    #[arg(short = 's', long)]
    synced_only: bool,

    /// Batas toleransi selisih durasi audio vs lirik dalam detik
    #[arg(short = 't', long, default_value = "3.0")]
    tolerance: f64,

    /// Jumlah pemrosesan file bersamaan / multi-thread (default: kapasitas core mesin)
    #[arg(short = 'c', long, default_value_t = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8))]
    concurrency: usize,

    /// Uji coba pencarian tanpa memodifikasi tag file audio di disk
    #[arg(short = 'd', long)]
    dry_run: bool,

    /// Tampilkan detail log proses per file
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Jangan upgrade file yang sudah memiliki lirik teks ke synced lirik (default: otomatis di-upgrade)
    #[arg(long)]
    no_upgrade_plain: bool,

    /// Langsung jalankan proses Fetch Lyrics tanpa prompt interaktif
    #[arg(short = 'y', long)]
    yes: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let options = ProcessOptions {
        overwrite: cli.overwrite,
        synced_only: cli.synced_only,
        upgrade_plain: !cli.no_upgrade_plain,
        tolerance_secs: cli.tolerance,
        dry_run: cli.dry_run,
        verbose: cli.verbose,
        concurrency: cli.concurrency.max(1),
    };

    let target_path = if cli.path.exists() {
        cli.path.canonicalize().unwrap_or(cli.path.clone())
    } else {
        cli.path.clone()
    };

    if !target_path.exists() {
        eprintln!("{}: Path '{}' tidak ditemukan!", "Error".red().bold(), target_path.display());
        std::process::exit(1);
    }

    loop {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message("Memindai direktori...");

        let tracks = scan_directory_with_progress(&target_path, cli.recursive, |path, found| {
            if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                spinner.set_message(format!("Memindai ({} file ditemukan): {}", found, name));
            }
        })?;
        spinner.finish_and_clear();

        // Tahap 1: Tampilkan tabel daftar lagu
        print_track_list(&tracks, &target_path);

        if tracks.is_empty() {
            break;
        }

        // Tahap 2: Tombol / Aksi Fetch Lyrics
        let action = if cli.yes {
            UserAction::FetchLyrics
        } else {
            prompt_user_action()?
        };

        match action {
            UserAction::FetchLyrics => {
                run_fetch_and_embed(&tracks, &options).await?;
                break;
            }
            UserAction::Rescan => {
                println!("{}", "Memindai ulang direktori...".yellow());
                continue;
            }
            UserAction::Quit => {
                println!("{}", "Operasi dibatalkan oleh pengguna.".dimmed());
                break;
            }
        }
    }

    Ok(())
}
