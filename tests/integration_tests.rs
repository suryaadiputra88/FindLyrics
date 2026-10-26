use findlyrics::models::{AudioFormat, LyricState, LyricsPayload, TrackItem};
use std::path::PathBuf;

#[test]
fn test_audio_format_extensions() {
    assert_eq!(AudioFormat::from_extension("mp3"), AudioFormat::Mp3);
    assert_eq!(AudioFormat::from_extension("MP3"), AudioFormat::Mp3);
    assert_eq!(AudioFormat::from_extension("flac"), AudioFormat::Flac);
    assert_eq!(AudioFormat::from_extension("wav"), AudioFormat::Wav);
    assert_eq!(AudioFormat::from_extension("wave"), AudioFormat::Wav);
    assert_eq!(AudioFormat::from_extension("m4a"), AudioFormat::M4a);
    assert_eq!(AudioFormat::from_extension("ogg"), AudioFormat::Ogg);
    assert_eq!(AudioFormat::from_extension("opus"), AudioFormat::Opus);
    assert_eq!(AudioFormat::from_extension("txt"), AudioFormat::Unknown);
    assert_eq!(AudioFormat::from_extension(""), AudioFormat::Unknown);
}

#[test]
fn test_track_item_formatting() {
    let track = TrackItem {
        path: PathBuf::from("/Music/test.flac"),
        filename: "test.flac".to_string(),
        format: AudioFormat::Flac,
        title: Some("Bohemian Rhapsody".to_string()),
        artist: Some("Queen".to_string()),
        album: Some("A Night at the Opera".to_string()),
        duration_secs: 354.2,
        lyric_state: LyricState::None,
        current_lyric_preview: None,
    };

    assert_eq!(track.display_title(), "Bohemian Rhapsody");
    assert_eq!(track.display_artist(), "Queen");
    assert_eq!(track.formatted_duration(), "05:54");
}

#[test]
fn test_lyrics_payload_selection() {
    let synced = "[00:10.00] Synced line".to_string();
    let plain = "Plain line".to_string();

    let payload_both = LyricsPayload {
        provider: "LRCLIB".to_string(),
        is_synced: true,
        plain_lyrics: Some(plain.clone()),
        synced_lyrics: Some(synced.clone()),
        duration_diff: 0.1,
    };

    assert_eq!(payload_both.best_lyrics(), Some(synced.as_str()));

    let payload_plain_only = LyricsPayload {
        provider: "LRCLIB".to_string(),
        is_synced: false,
        plain_lyrics: Some(plain.clone()),
        synced_lyrics: None,
        duration_diff: 0.5,
    };

    assert_eq!(payload_plain_only.best_lyrics(), Some(plain.as_str()));
}
