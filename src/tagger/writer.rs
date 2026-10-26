use std::path::Path;
use anyhow::{Context, Result};
use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};

pub fn embed_lyrics_to_file(path: &Path, lyrics: &str) -> Result<()> {
    let mut tagged_file = Probe::open(path)
        .with_context(|| format!("Failed to open audio file: {:?}", path))?
        .read()
        .with_context(|| format!("Failed to read tags from: {:?}", path))?;

    let tag_type = tagged_file.primary_tag_type();

    let tag = match tagged_file.primary_tag_mut() {
        Some(primary) => primary,
        None => {
            if let Some(first) = tagged_file.first_tag_mut() {
                first
            } else {
                tagged_file.insert_tag(Tag::new(tag_type));
                tagged_file.primary_tag_mut().expect("Inserted tag must exist")
            }
        }
    };

    tag.insert_text(ItemKey::Lyrics, lyrics.to_string());
    tag.save_to_path(path, WriteOptions::default())
        .with_context(|| format!("Failed to save tag to file: {:?}", path))?;

    Ok(())
}
