use std::path::Path;
use anyhow::{Context, Result};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::ItemKey;

use crate::models::{AudioFormat, LyricState, TrackItem};

pub fn has_lrc_timestamps(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with('[')
            && trimmed.len() >= 8
            && trimmed.chars().nth(1).map_or(false, |c| c.is_ascii_digit())
            && trimmed.contains(':')
            && trimmed.contains(']')
    })
}

pub fn read_track_metadata(path: &Path) -> Result<TrackItem> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let format = AudioFormat::from_extension(ext);

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let tagged_file = Probe::open(path)
        .with_context(|| format!("Failed to open audio file: {:?}", path))?
        .read()
        .with_context(|| format!("Failed to read metadata from: {:?}", path))?;

    let properties = tagged_file.properties();
    let duration_secs = properties.duration().as_secs_f64();

    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    let (title, artist, album, raw_lyrics) = if let Some(tag) = tag {
        let title = tag.get_string(&ItemKey::TrackTitle).map(|s| s.to_string());
        let artist = tag.get_string(&ItemKey::TrackArtist).map(|s| s.to_string());
        let album = tag.get_string(&ItemKey::AlbumTitle).map(|s| s.to_string());
        let raw_lyrics = tag.get_string(&ItemKey::Lyrics).map(|s| s.to_string());
        (title, artist, album, raw_lyrics)
    } else {
        (None, None, None, None)
    };

    // Determine lyric state: check tag first, then check sidecar file (.lrc / .txt)
    let (lyric_state, current_lyric_preview) = match raw_lyrics {
        Some(ref text) if !text.trim().is_empty() => {
            let is_synced = has_lrc_timestamps(text);
            let st = if is_synced {
                LyricState::Synced
            } else {
                LyricState::Plain
            };
            let prev = text.lines().next().map(|l| l.chars().take(60).collect());
            (st, prev)
        }
        _ => {
            // Check for sidecar files: .lrc or .txt in the same directory
            let parent = path.parent();
            let stem = path.file_stem().and_then(|s| s.to_str());

            if let (Some(parent), Some(stem)) = (parent, stem) {
                let lrc_file = parent.join(format!("{}.lrc", stem));
                let txt_file = parent.join(format!("{}.txt", stem));

                if lrc_file.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&lrc_file) {
                        if !content.trim().is_empty() {
                            let is_synced = has_lrc_timestamps(&content);
                            let prev = content.lines().next().map(|l| l.chars().take(60).collect());
                            (if is_synced { LyricState::Synced } else { LyricState::Plain }, prev)
                        } else {
                            (LyricState::None, None)
                        }
                    } else {
                        (LyricState::None, None)
                    }
                } else if txt_file.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&txt_file) {
                        if !content.trim().is_empty() {
                            let is_synced = has_lrc_timestamps(&content);
                            let prev = content.lines().next().map(|l| l.chars().take(60).collect());
                            (if is_synced { LyricState::Synced } else { LyricState::Plain }, prev)
                        } else {
                            (LyricState::None, None)
                        }
                    } else {
                        (LyricState::None, None)
                    }
                } else {
                    (LyricState::None, None)
                }
            } else {
                (LyricState::None, None)
            }
        }
    };

    Ok(TrackItem {
        path: path.to_path_buf(),
        filename,
        format,
        title,
        artist,
        album,
        duration_secs,
        lyric_state,
        current_lyric_preview,
    })
}
