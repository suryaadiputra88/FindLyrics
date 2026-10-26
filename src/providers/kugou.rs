use anyhow::{Context, Result};
use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::time::Duration;

use crate::matcher::{clean_title, is_duration_within_tolerance};
use crate::models::{LyricsPayload, TrackItem};

const KUGOU_SEARCH_URL: &str = "http://krcs.kugou.com/search";
const KUGOU_DOWNLOAD_URL: &str = "http://krcs.kugou.com/download";
const KUGOU_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KugouSearchResponse {
    status: Option<i32>,
    candidates: Option<Vec<KugouCandidate>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KugouCandidate {
    id: Option<String>,
    accesskey: Option<String>,
    singer: Option<String>,
    song: Option<String>,
    duration: Option<f64>, // in milliseconds
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KugouDownloadResponse {
    status: Option<i32>,
    content: Option<String>,
}

pub struct KugouClient {
    client: reqwest::Client,
}

impl Default for KugouClient {
    fn default() -> Self {
        Self::new()
    }
}

impl KugouClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(KUGOU_USER_AGENT));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build reqwest client for Kugou");

        Self { client }
    }

    pub async fn fetch_lyrics(
        &self,
        track: &TrackItem,
        tolerance_secs: f64,
    ) -> Result<Option<LyricsPayload>> {
        let title = match track.title.as_deref() {
            Some(t) if !t.trim().is_empty() => t,
            _ => return Ok(None),
        };
        let artist = track.artist.as_deref().unwrap_or("");
        let cleaned_title = clean_title(title);

        let query = if !artist.is_empty() {
            format!("{} {}", cleaned_title, artist)
        } else {
            cleaned_title
        };

        let duration_ms = if track.duration_secs > 0.0 {
            (track.duration_secs * 1000.0).round() as u64
        } else {
            0
        };

        let mut url = format!(
            "{}?ver=1&man=yes&client=mobi&keyword={}",
            KUGOU_SEARCH_URL,
            urlencoding(&query)
        );
        if duration_ms > 0 {
            url.push_str(&format!("&duration={}", duration_ms));
        }

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }

        let search_data: KugouSearchResponse = resp.json().await.unwrap_or(KugouSearchResponse {
            status: None,
            candidates: None,
        });

        let candidates = match search_data.candidates {
            Some(c) if !c.is_empty() => c,
            _ => return Ok(None),
        };

        // Pick best candidate by duration tolerance
        let mut best_candidate: Option<(&KugouCandidate, f64)> = None;

        for cand in &candidates {
            if let (Some(ref id), Some(ref key)) = (&cand.id, &cand.accesskey) {
                if id.is_empty() || key.is_empty() {
                    continue;
                }

                let cand_dur_secs = cand.duration.unwrap_or(0.0) / 1000.0;
                let diff = track.duration_secs - cand_dur_secs;

                if track.duration_secs > 0.0 && cand_dur_secs > 0.0 {
                    if !is_duration_within_tolerance(track.duration_secs, cand_dur_secs, tolerance_secs) {
                        continue;
                    }
                }

                match best_candidate {
                    Some((_, current_min)) if diff.abs() < current_min => {
                        best_candidate = Some((cand, diff.abs()));
                    }
                    None => {
                        best_candidate = Some((cand, diff.abs()));
                    }
                    _ => {}
                }
            }
        }

        let (chosen, _) = match best_candidate {
            Some(pair) => pair,
            None => {
                // If duration match failed or track duration is 0, pick the first candidate
                if let Some(first) = candidates.first() {
                    (first, 0.0)
                } else {
                    return Ok(None);
                }
            }
        };

        let id = match chosen.id.as_deref() {
            Some(id) => id,
            None => return Ok(None),
        };
        let accesskey = match chosen.accesskey.as_deref() {
            Some(key) => key,
            None => return Ok(None),
        };

        // Download lyrics content
        let dl_url = format!(
            "{}?ver=1&client=pc&id={}&accesskey={}&fmt=lrc&charset=utf8",
            KUGOU_DOWNLOAD_URL, id, accesskey
        );

        let dl_resp = self.client.get(&dl_url).send().await?;
        if !dl_resp.status().is_success() {
            return Ok(None);
        }

        let dl_data: KugouDownloadResponse = dl_resp.json().await.unwrap_or(KugouDownloadResponse {
            status: None,
            content: None,
        });

        let base64_content = match dl_data.content {
            Some(c) if !c.trim().is_empty() => c,
            _ => return Ok(None),
        };

        let decoded_bytes = BASE64_STANDARD
            .decode(base64_content.trim())
            .context("Failed to base64 decode Kugou lyric")?;

        let lyric_str = String::from_utf8_lossy(&decoded_bytes).to_string();
        if lyric_str.trim().is_empty() {
            return Ok(None);
        }

        let cand_dur_secs = chosen.duration.unwrap_or(0.0) / 1000.0;
        let duration_diff = track.duration_secs - cand_dur_secs;

        Ok(Some(LyricsPayload {
            provider: "Kugou".to_string(),
            is_synced: true,
            plain_lyrics: None,
            synced_lyrics: Some(lyric_str),
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
