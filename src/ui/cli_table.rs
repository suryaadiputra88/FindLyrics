use std::path::Path;
use anyhow::Result;
use colored::Colorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, Table};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::models::{LyricState, TrackItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    FetchLyrics,
    Rescan,
    Quit,
}

pub fn print_track_list(tracks: &[TrackItem], dir_path: &Path) {
    println!("\n{}", "=========================================================================================".cyan().bold());
    println!("{}", "                        FINDLYRICS - MUSIC TRACK LIST                           ".cyan().bold());
    println!("{}\n", "=========================================================================================".cyan().bold());
    println!("📁 {}: {}\n", "Direktori".bold(), dir_path.display().to_string().yellow());

    if tracks.is_empty() {
        println!("{}", "Tidak ada file musik yang didukung ditemukan di direktori ini.".yellow());
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").set_alignment(CellAlignment::Center),
            Cell::new("Format").set_alignment(CellAlignment::Center),
            Cell::new("Durasi").set_alignment(CellAlignment::Center),
            Cell::new("Judul Lagu"),
            Cell::new("Artis"),
            Cell::new("Status Lirik").set_alignment(CellAlignment::Center),
        ]);

    let mut missing_count = 0;
    let mut has_count = 0;

    for (idx, track) in tracks.iter().enumerate() {
        let lyric_cell = match track.lyric_state {
            LyricState::None => {
                missing_count += 1;
                Cell::new("Belum Ada").fg(Color::Yellow)
            }
            LyricState::Plain => {
                has_count += 1;
                Cell::new("Plain Text").fg(Color::Blue)
            }
            LyricState::Synced => {
                has_count += 1;
                Cell::new("Synced (LRC)").fg(Color::Green)
            }
        };

        table.add_row(vec![
            Cell::new(idx + 1).set_alignment(CellAlignment::Center),
            Cell::new(track.format.as_str()).set_alignment(CellAlignment::Center).fg(Color::Cyan),
            Cell::new(track.formatted_duration()).set_alignment(CellAlignment::Center),
            Cell::new(track.display_title()),
            Cell::new(track.display_artist()),
            lyric_cell,
        ]);
    }

    println!("{table}");
    println!(
        "\nTotal: {} file | {} {} | {} {}\n",
        tracks.len().to_string().bold(),
        missing_count.to_string().yellow().bold(),
        "belum memiliki lirik",
        has_count.to_string().green().bold(),
        "sudah memiliki lirik"
    );
}

pub fn prompt_user_action() -> Result<UserAction> {
    let items = vec![
        "▶ Fetch Lyrics (Download & Langsung Embed ke Audio Tags)",
        "🔄 Rescan Direktori",
        "❌ Keluar",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pilih Aksi Selanjutnya:")
        .default(0)
        .items(&items)
        .interact()?;

    match selection {
        0 => Ok(UserAction::FetchLyrics),
        1 => Ok(UserAction::Rescan),
        _ => Ok(UserAction::Quit),
    }
}
