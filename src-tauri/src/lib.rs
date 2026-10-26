use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Emitter;

use findlyrics::models::LyricState;
use findlyrics::providers::MultiProviderClient;
use findlyrics::scanner::scan_directory_with_progress;
use findlyrics::tagger::{embed_lyrics_to_file, read_track_metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgressPayload {
    pub current_file: String,
    pub found_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackItemDto {
    pub path: String,
    pub filename: String,
    pub format: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub duration_formatted: String,
    pub lyric_state: String,
    pub lyric_preview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedResultDto {
    pub success: bool,
    pub is_synced: bool,
    pub provider: String,
    pub message: String,
    pub duration_diff: f64,
    pub lyrics_preview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchStepResultDto {
    pub action: String, // "embed", "skip", "notfound", "error"
    pub is_synced: bool,
    pub provider: String,
    pub message: String,
    pub duration_diff: f64,
    pub lyrics: Option<String>,
    pub lyrics_preview: Option<String>,
    pub is_upgrade: bool,
}

#[tauri::command]
async fn scan_music_directory(
    app: tauri::AppHandle,
    path: String,
    recursive: bool,
) -> Result<Vec<TrackItemDto>, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err("Direktori tidak ditemukan".to_string());
    }

    let tracks = scan_directory_with_progress(p, recursive, |file_path, found| {
        let name = file_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_string();
        let _ = app.emit(
            "scan-progress",
            ScanProgressPayload {
                current_file: name,
                found_count: found,
            },
        );
    })
    .map_err(|e| e.to_string())?;

    let dtos = tracks
        .into_iter()
        .map(|t| {
            let state_str = match t.lyric_state {
                LyricState::None => "none",
                LyricState::Plain => "plain",
                LyricState::Synced => "synced",
            }
            .to_string();

            let title = t.display_title().to_string();
            let artist = t.display_artist().to_string();
            let duration_formatted = t.formatted_duration();
            let format = t.format.as_str().to_string();
            let album = t.album.unwrap_or_else(|| "-".to_string());
            let path_str = t.path.to_string_lossy().to_string();
            let filename = t.filename;

            TrackItemDto {
                path: path_str,
                filename,
                format,
                title,
                artist,
                album,
                duration: t.duration_secs,
                duration_formatted,
                lyric_state: state_str,
                lyric_preview: t.current_lyric_preview,
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
async fn fetch_and_embed_track(
    path: String,
    tolerance_secs: f64,
    synced_only: bool,
    overwrite: bool,
    upgrade_plain: bool,
) -> Result<EmbedResultDto, String> {
    let p = Path::new(&path);
    let track = read_track_metadata(p).map_err(|e| e.to_string())?;

    let is_synced = track.lyric_state == LyricState::Synced;
    let is_plain = track.lyric_state == LyricState::Plain;

    if is_synced && !overwrite {
        return Ok(EmbedResultDto {
            success: true,
            is_synced: true,
            provider: "Local".to_string(),
            message: "Dilewati (sudah memiliki synced lyrics)".to_string(),
            duration_diff: 0.0,
            lyrics_preview: track.current_lyric_preview,
        });
    }

    if is_plain && !overwrite && !upgrade_plain {
        return Ok(EmbedResultDto {
            success: true,
            is_synced: false,
            provider: "Local".to_string(),
            message: "Dilewati (sudah memiliki lirik teks)".to_string(),
            duration_diff: 0.0,
            lyrics_preview: track.current_lyric_preview,
        });
    }

    if track.title.is_none() {
        return Ok(EmbedResultDto {
            success: false,
            is_synced: false,
            provider: "None".to_string(),
            message: "Metadata judul kosong".to_string(),
            duration_diff: 0.0,
            lyrics_preview: None,
        });
    }

    let client = MultiProviderClient::new();
    let require_synced = synced_only || (is_plain && !overwrite);

    match client.fetch_lyrics(&track, tolerance_secs, require_synced).await {
        Ok(Some(payload)) => {
            if let Some(best_lyrics) = payload.best_lyrics() {
                embed_lyrics_to_file(p, best_lyrics).map_err(|e| e.to_string())?;
                let snippet: String = best_lyrics.lines().take(3).collect::<Vec<_>>().join("\n");

                let msg = if is_plain && payload.is_synced {
                    format!("Berhasil di-upgrade ke Synced Lyrics via {}", payload.provider)
                } else if payload.is_synced {
                    format!("Berhasil embed Synced Lyrics via {}", payload.provider)
                } else {
                    format!("Berhasil embed Plain Lyrics via {}", payload.provider)
                };

                Ok(EmbedResultDto {
                    success: true,
                    is_synced: payload.is_synced,
                    provider: payload.provider.clone(),
                    message: msg,
                    duration_diff: payload.duration_diff,
                    lyrics_preview: Some(snippet),
                })
            } else {
                Ok(EmbedResultDto {
                    success: false,
                    is_synced: false,
                    provider: payload.provider,
                    message: "Konten lirik kosong dari provider".to_string(),
                    duration_diff: 0.0,
                    lyrics_preview: None,
                })
            }
        }
        Ok(None) => {
            if is_plain {
                Ok(EmbedResultDto {
                    success: true,
                    is_synced: false,
                    provider: "Local".to_string(),
                    message: "Lirik synced tidak ditemukan, mempertahankan lirik teks saat ini".to_string(),
                    duration_diff: 0.0,
                    lyrics_preview: track.current_lyric_preview,
                })
            } else {
                Ok(EmbedResultDto {
                    success: false,
                    is_synced: false,
                    provider: "None".to_string(),
                    message: "Lirik tidak ditemukan di seluruh provider".to_string(),
                    duration_diff: 0.0,
                    lyrics_preview: None,
                })
            }
        }
        Err(e) => Err(format!("Error koneksi provider: {}", e)),
    }
}

#[tauri::command]
async fn fetch_track_lyrics(
    path: String,
    tolerance_secs: f64,
    synced_only: bool,
    overwrite: bool,
    upgrade_plain: bool,
) -> Result<FetchStepResultDto, String> {
    let p = Path::new(&path);
    let track = read_track_metadata(p).map_err(|e| e.to_string())?;

    let is_synced = track.lyric_state == LyricState::Synced;
    let is_plain = track.lyric_state == LyricState::Plain;

    if is_synced && !overwrite {
        return Ok(FetchStepResultDto {
            action: "skip".to_string(),
            is_synced: true,
            provider: "Local".to_string(),
            message: "Dilewati (sudah memiliki synced lyrics)".to_string(),
            duration_diff: 0.0,
            lyrics: None,
            lyrics_preview: track.current_lyric_preview,
            is_upgrade: false,
        });
    }

    if is_plain && !overwrite && !upgrade_plain {
        return Ok(FetchStepResultDto {
            action: "skip".to_string(),
            is_synced: false,
            provider: "Local".to_string(),
            message: "Dilewati (sudah memiliki lirik teks)".to_string(),
            duration_diff: 0.0,
            lyrics: None,
            lyrics_preview: track.current_lyric_preview,
            is_upgrade: false,
        });
    }

    if track.title.is_none() {
        return Ok(FetchStepResultDto {
            action: "error".to_string(),
            is_synced: false,
            provider: "None".to_string(),
            message: "Metadata judul kosong".to_string(),
            duration_diff: 0.0,
            lyrics: None,
            lyrics_preview: None,
            is_upgrade: false,
        });
    }

    let client = MultiProviderClient::new();
    let require_synced = synced_only || (is_plain && !overwrite);

    match client.fetch_lyrics(&track, tolerance_secs, require_synced).await {
        Ok(Some(payload)) => {
            if let Some(best_lyrics) = payload.best_lyrics() {
                let snippet: String = best_lyrics.lines().take(3).collect::<Vec<_>>().join("\n");
                let is_upgrade = is_plain && payload.is_synced;
                Ok(FetchStepResultDto {
                    action: "embed".to_string(),
                    is_synced: payload.is_synced,
                    provider: payload.provider.clone(),
                    message: format!("Lirik ditemukan via {}", payload.provider),
                    duration_diff: payload.duration_diff,
                    lyrics: Some(best_lyrics.to_string()),
                    lyrics_preview: Some(snippet),
                    is_upgrade,
                })
            } else {
                Ok(FetchStepResultDto {
                    action: "notfound".to_string(),
                    is_synced: false,
                    provider: payload.provider,
                    message: "Konten lirik kosong dari provider".to_string(),
                    duration_diff: 0.0,
                    lyrics: None,
                    lyrics_preview: None,
                    is_upgrade: false,
                })
            }
        }
        Ok(None) => {
            if is_plain {
                Ok(FetchStepResultDto {
                    action: "skip".to_string(),
                    is_synced: false,
                    provider: "Local".to_string(),
                    message: "Lirik synced tidak ditemukan, mempertahankan lirik teks saat ini".to_string(),
                    duration_diff: 0.0,
                    lyrics: None,
                    lyrics_preview: track.current_lyric_preview,
                    is_upgrade: false,
                })
            } else {
                Ok(FetchStepResultDto {
                    action: "notfound".to_string(),
                    is_synced: false,
                    provider: "None".to_string(),
                    message: "Lirik tidak ditemukan di seluruh provider".to_string(),
                    duration_diff: 0.0,
                    lyrics: None,
                    lyrics_preview: None,
                    is_upgrade: false,
                })
            }
        }
        Err(e) => Err(format!("Error koneksi provider: {}", e)),
    }
}

#[tauri::command]
async fn embed_track_lyrics(
    path: String,
    lyrics: String,
    provider: String,
    is_synced: bool,
    is_upgrade: bool,
) -> Result<EmbedResultDto, String> {
    let p = Path::new(&path);
    embed_lyrics_to_file(p, &lyrics).map_err(|e| e.to_string())?;

    let snippet: String = lyrics.lines().take(3).collect::<Vec<_>>().join("\n");
    let msg = if is_upgrade {
        format!("Berhasil ditanamkan (Upgrade ke Synced via {})", provider)
    } else if is_synced {
        format!("Berhasil ditanamkan (Synced Lyrics via {})", provider)
    } else {
        format!("Berhasil ditanamkan (Plain Lyrics via {})", provider)
    };

    Ok(EmbedResultDto {
        success: true,
        is_synced,
        provider,
        message: msg,
        duration_diff: 0.0,
        lyrics_preview: Some(snippet),
    })
}

#[tauri::command]
async fn get_track_lyrics(path: String) -> Result<Option<String>, String> {
    let p = Path::new(&path);
    let track = read_track_metadata(p).map_err(|e| e.to_string())?;

    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let tagged_file = Probe::open(p)
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;

    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
    let lyrics = tag.and_then(|t| t.get_string(&ItemKey::Lyrics).map(|s| s.to_string()));

    Ok(lyrics.or(track.current_lyric_preview))
}

#[tauri::command]
fn get_system_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_music_directory,
            fetch_and_embed_track,
            fetch_track_lyrics,
            embed_track_lyrics,
            get_track_lyrics,
            get_system_concurrency
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
