use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::time::Duration;

use crate::matcher::{clean_title, is_duration_within_tolerance};
use crate::models::{LyricsPayload, TrackItem};

const BASE_URL: &str = "https://lrclib.net/api";
const APP_USER_AGENT: &str = "FindLyrics/0.1.0 (https://github.com/bsa/FindLyrics)";

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    pub id: Option<u64>,
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

pub struct LrclibClient {
    client: reqwest::Client,
}

impl Default for LrclibClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LrclibClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(APP_USER_AGENT));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(12))
            .build()
            .expect("Failed to build reqwest client");

        Self { client }
    }

    pub async fn fetch_lyrics(
        &self,
        track: &TrackItem,
        tolerance_secs: f64,
        synced_only: bool,
    ) -> Result<Option<LyricsPayload>> {
        let title = match track.title.as_deref() {
            Some(t) if !t.trim().is_empty() => t,
            _ => return Ok(None),
        };
        let artist = track.artist.as_deref().unwrap_or("");
        let album = track.album.as_deref().unwrap_or("");
        let cleaned_title = clean_title(title);

        // 1. Direct GET attempt
        if let Ok(Some(payload)) = self
            .get_direct(&cleaned_title, artist, album, track.duration_secs, tolerance_secs, synced_only)
            .await
        {
            return Ok(Some(payload));
        }

        // 2. Fallback to Search API
        self.search_fallback(&cleaned_title, artist, track.duration_secs, tolerance_secs, synced_only)
            .await
    }

    async fn get_direct(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration_secs: f64,
        tolerance_secs: f64,
        synced_only: bool,
    ) -> Result<Option<LyricsPayload>> {
        let mut url = format!("{}/get?track_name={}", BASE_URL, urlencoding(title));
        if !artist.is_empty() {
            url.push_str(&format!("&artist_name={}", urlencoding(artist)));
        }
        if !album.is_empty() {
            url.push_str(&format!("&album_name={}", urlencoding(album)));
        }
        if duration_secs > 0.0 {
            url.push_str(&format!("&duration={}", duration_secs.round() as u64));
        }

        let resp = self.client.get(&url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            return Ok(None);
        }

        let body: LrclibResponse = resp.json().await?;
        self.process_response(body, duration_secs, tolerance_secs, synced_only)
    }

    async fn search_fallback(
        &self,
        title: &str,
        artist: &str,
        duration_secs: f64,
        tolerance_secs: f64,
        synced_only: bool,
    ) -> Result<Option<LyricsPayload>> {
        let mut url = format!("{}/search?track_name={}", BASE_URL, urlencoding(title));
        if !artist.is_empty() {
            url.push_str(&format!("&artist_name={}", urlencoding(artist)));
        }

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }

        let results: Vec<LrclibResponse> = resp.json().await.unwrap_or_default();

        let mut best_match: Option<(LyricsPayload, f64)> = None;

        for item in results {
            if let Ok(Some(payload)) = self.process_response(item, duration_secs, tolerance_secs, synced_only) {
                let diff = payload.duration_diff.abs();
                match best_match {
                    Some((_, current_min)) if diff < current_min => {
                        best_match = Some((payload, diff));
                    }
                    None => {
                        best_match = Some((payload, diff));
                    }
                    _ => {}
                }
            }
        }

        Ok(best_match.map(|(p, _)| p))
    }

    fn process_response(
        &self,
        item: LrclibResponse,
        actual_duration: f64,
        tolerance_secs: f64,
        synced_only: bool,
    ) -> Result<Option<LyricsPayload>> {
        let has_synced = item.synced_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_plain = item.plain_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);

        if synced_only && !has_synced {
            return Ok(None);
        }

        if !has_synced && !has_plain {
            return Ok(None);
        }

        let candidate_duration = item.duration.unwrap_or(actual_duration);
        let duration_diff = actual_duration - candidate_duration;

        if actual_duration > 0.0 && !is_duration_within_tolerance(actual_duration, candidate_duration, tolerance_secs) {
            return Ok(None);
        }

        Ok(Some(LyricsPayload {
            provider: "LRCLIB".to_string(),
            is_synced: has_synced,
            plain_lyrics: item.plain_lyrics,
            synced_lyrics: item.synced_lyrics,
            duration_diff,
        }))
    }
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}
