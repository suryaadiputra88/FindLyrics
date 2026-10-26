use anyhow::Result;

use crate::models::{LyricsPayload, TrackItem};
use crate::providers::kugou::KugouClient;
use crate::providers::lrclib::LrclibClient;
use crate::providers::lyricsovh::LyricsOvhClient;

pub struct MultiProviderClient {
    lrclib: LrclibClient,
    kugou: KugouClient,
    lyricsovh: LyricsOvhClient,
}

impl Default for MultiProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiProviderClient {
    pub fn new() -> Self {
        Self {
            lrclib: LrclibClient::new(),
            kugou: KugouClient::new(),
            lyricsovh: LyricsOvhClient::new(),
        }
    }

    pub async fn fetch_lyrics(
        &self,
        track: &TrackItem,
        tolerance_secs: f64,
        synced_only: bool,
    ) -> Result<Option<LyricsPayload>> {
        // 1. Prioritas 1: LRCLIB (Open-source, synced + plain)
        if let Ok(Some(payload)) = self.lrclib.fetch_lyrics(track, tolerance_secs, synced_only).await {
            return Ok(Some(payload));
        }

        // 2. Prioritas 2: Kugou Music (Database raksasa untuk synced LRC)
        if let Ok(Some(payload)) = self.kugou.fetch_lyrics(track, tolerance_secs).await {
            return Ok(Some(payload));
        }

        // 3. Prioritas 3: Lyrics.ovh (Fallback plain text jika synced_only false)
        if !synced_only {
            if let Ok(Some(payload)) = self.lyricsovh.fetch_lyrics(track).await {
                return Ok(Some(payload));
            }
        }

        Ok(None)
    }
}
