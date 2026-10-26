use std::path::Path;
use anyhow::Result;
use walkdir::WalkDir;

use crate::models::{AudioFormat, TrackItem};
use crate::tagger::read_track_metadata;

pub fn scan_directory_with_progress<F>(
    dir_path: &Path,
    recursive: bool,
    mut on_progress: F,
) -> Result<Vec<TrackItem>>
where
    F: FnMut(&Path, usize),
{
    let mut tracks = Vec::new();

    if dir_path.is_file() {
        if let Some(ext) = dir_path.extension().and_then(|e| e.to_str()) {
            if AudioFormat::from_extension(ext).is_supported() {
                if let Ok(track) = read_track_metadata(dir_path) {
                    tracks.push(track);
                    on_progress(dir_path, tracks.len());
                }
            }
        }
        return Ok(tracks);
    }

    let max_depth = if recursive { usize::MAX } else { 1 };

    for entry in WalkDir::new(dir_path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let format = AudioFormat::from_extension(ext);
                if format.is_supported() {
                    match read_track_metadata(path) {
                        Ok(track) => {
                            tracks.push(track);
                            on_progress(path, tracks.len());
                        }
                        Err(e) => {
                            eprintln!("Warning: Gagal membaca metadata {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    tracks.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    Ok(tracks)
}

pub fn scan_directory(dir_path: &Path, recursive: bool) -> Result<Vec<TrackItem>> {
    scan_directory_with_progress(dir_path, recursive, |_, _| {})
}
