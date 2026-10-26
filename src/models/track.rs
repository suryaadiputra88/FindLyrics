use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Wav,
    M4a,
    Ogg,
    Opus,
    Unknown,
}

impl AudioFormat {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "mp3" => AudioFormat::Mp3,
            "flac" => AudioFormat::Flac,
            "wav" | "wave" => AudioFormat::Wav,
            "m4a" | "aac" | "alac" => AudioFormat::M4a,
            "ogg" | "oga" => AudioFormat::Ogg,
            "opus" => AudioFormat::Opus,
            _ => AudioFormat::Unknown,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, AudioFormat::Unknown)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Wav => "WAV",
            AudioFormat::M4a => "M4A",
            AudioFormat::Ogg => "OGG",
            AudioFormat::Opus => "OPUS",
            AudioFormat::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricState {
    None,
    Plain,
    Synced,
}

impl LyricState {
    pub fn as_str(&self) -> &'static str {
        match self {
            LyricState::None => "No Lyrics",
            LyricState::Plain => "Plain",
            LyricState::Synced => "Synced (LRC)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackItem {
    pub path: PathBuf,
    pub filename: String,
    pub format: AudioFormat,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub lyric_state: LyricState,
    pub current_lyric_preview: Option<String>,
}

impl TrackItem {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.filename)
    }

    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("Unknown Artist")
    }

    pub fn formatted_duration(&self) -> String {
        let total_secs = self.duration_secs.round() as u64;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Debug, Clone)]
pub struct LyricsPayload {
    pub provider: String,
    pub is_synced: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub duration_diff: f64,
}

impl LyricsPayload {
    pub fn best_lyrics(&self) -> Option<&str> {
        if let Some(ref synced) = self.synced_lyrics {
            if !synced.trim().is_empty() {
                return Some(synced);
            }
        }
        self.plain_lyrics.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub overwrite: bool,
    pub synced_only: bool,
    pub upgrade_plain: bool,
    pub tolerance_secs: f64,
    pub dry_run: bool,
    pub verbose: bool,
    pub concurrency: usize,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        let default_concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8);
        Self {
            overwrite: false,
            synced_only: false,
            upgrade_plain: true,
            tolerance_secs: 3.0,
            dry_run: false,
            verbose: false,
            concurrency: default_concurrency,
        }
    }
}
