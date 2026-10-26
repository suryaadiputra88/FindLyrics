use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::time::Duration;

use crate::matcher::clean_title;
use crate::models::{LyricsPayload, TrackItem};

const BASE_URL: &str = "https://api.lyrics.ovh/v1";
const APP_USER_AGENT: &str = "FindLyrics/0.1.0 (https://github.com/bsa/FindLyrics)";

#[derive(Debug, Deserialize)]
struct LyricsOvhResponse {
    lyrics: Option<String>,
}

pub struct LyricsOvhClient {
    client: reqwest::Client,
}

impl Default for LyricsOvhClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsOvhClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(APP_USER_AGENT));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(8))
            .build()
            .expect("Failed to build reqwest client for Lyrics.ovh");

        Self { client }
    }

    pub async fn fetch_lyrics(&self, track: &TrackItem) -> Result<Option<LyricsPayload>> {
        let title = match track.title.as_deref() {
            Some(t) if !t.trim().is_empty() => t,
            _ => return Ok(None),
        };
        let artist = match track.artist.as_deref() {
            Some(a) if !a.trim().is_empty() => a,
            _ => return Ok(None),
        };

        let cleaned_title = clean_title(title);
        let url = format!(
            "{}/{}/{}",
            BASE_URL,
            urlencoding(artist),
            urlencoding(&cleaned_title)
        );

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }

        let data: LyricsOvhResponse = resp.json().await.unwrap_or(LyricsOvhResponse { lyrics: None });
        match data.lyrics {
            Some(text) if !text.trim().is_empty() => Ok(Some(LyricsPayload {
                provider: "Lyrics.ovh".to_string(),
                is_synced: false,
                plain_lyrics: Some(text),
                synced_lyrics: None,
                duration_diff: 0.0,
            })),
            _ => Ok(None),
        }
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
