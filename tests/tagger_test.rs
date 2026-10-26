use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

use findlyrics::matcher::{clean_title, is_duration_within_tolerance};
use findlyrics::models::{AudioFormat, LyricState};
use findlyrics::tagger::{embed_lyrics_to_file, has_lrc_timestamps, read_track_metadata};

/// Creates a valid PCM WAV file with 100 bytes of silence
fn create_dummy_wav(path: &Path) {
    let mut file = File::create(path).expect("Failed to create dummy wav");
    let data_len: u32 = 100;
    let riff_len: u32 = 36 + data_len;

    // RIFF chunk descriptor
    file.write_all(b"RIFF").unwrap();
    file.write_all(&riff_len.to_le_bytes()).unwrap(); // file size - 8
    file.write_all(b"WAVE").unwrap();

    // fmt sub-chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&(16u32).to_le_bytes()).unwrap(); // Subchunk1Size (16 for PCM)
    file.write_all(&(1u16).to_le_bytes()).unwrap(); // AudioFormat (1 for PCM)
    file.write_all(&(1u16).to_le_bytes()).unwrap(); // NumChannels (1 mono)
    file.write_all(&(44100u32).to_le_bytes()).unwrap(); // SampleRate
    file.write_all(&(88200u32).to_le_bytes()).unwrap(); // ByteRate
    file.write_all(&(2u16).to_le_bytes()).unwrap(); // BlockAlign
    file.write_all(&(16u16).to_le_bytes()).unwrap(); // BitsPerSample

    // data sub-chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_len.to_le_bytes()).unwrap();
    let silence = vec![0u8; data_len as usize];
    file.write_all(&silence).unwrap();
}

#[test]
fn test_audio_format_recognition() {
    assert_eq!(AudioFormat::from_extension("mp3"), AudioFormat::Mp3);
    assert_eq!(AudioFormat::from_extension("flac"), AudioFormat::Flac);
    assert_eq!(AudioFormat::from_extension("wav"), AudioFormat::Wav);
    assert_eq!(AudioFormat::from_extension("m4a"), AudioFormat::M4a);
    assert_eq!(AudioFormat::from_extension("ogg"), AudioFormat::Ogg);
    assert_eq!(AudioFormat::from_extension("opus"), AudioFormat::Opus);
    assert_eq!(AudioFormat::from_extension("txt"), AudioFormat::Unknown);
}

#[test]
fn test_clean_title() {
    assert_eq!(clean_title("Numb (Official Music Video)"), "Numb");
    assert_eq!(clean_title("Yesterday (Remastered 2009)"), "Yesterday");
    assert_eq!(clean_title("Imagine (Audio)"), "Imagine");
    assert_eq!(clean_title("Hello (Lyrics)"), "Hello");
}

#[test]
fn test_duration_tolerance() {
    assert!(is_duration_within_tolerance(180.0, 182.5, 3.0));
    assert!(is_duration_within_tolerance(180.0, 178.0, 3.0));
    assert!(!is_duration_within_tolerance(180.0, 185.0, 3.0));
}

#[test]
fn test_has_lrc_timestamps() {
    assert!(has_lrc_timestamps("[00:12.34] Hello World"));
    assert!(has_lrc_timestamps("[01:05.00] Line two\n[01:10.50] Line three"));
    assert!(!has_lrc_timestamps("This is just a plain lyrics text line"));
    assert!(!has_lrc_timestamps("Verse 1:\nNo timestamps anywhere"));
}

#[test]
fn test_wav_embed_and_read_lyrics() {
    let dir = tempdir().expect("Failed to create tempdir");
    let test_file = dir.path().join("test_song.wav");
    create_dummy_wav(&test_file);

    // Initial read: should have no lyrics
    let initial_track = read_track_metadata(&test_file).expect("Failed to read initial wav");
    assert_eq!(initial_track.lyric_state, LyricState::None);

    // Embed synced lyrics
    let sample_lrc = "[00:01.00] Line one\n[00:04.50] Line two\n[00:08.20] Line three";
    embed_lyrics_to_file(&test_file, sample_lrc).expect("Failed to embed lyrics into wav");

    // Read back: should now be Synced!
    let updated_track = read_track_metadata(&test_file).expect("Failed to read updated wav");
    assert_eq!(updated_track.lyric_state, LyricState::Synced);
}

#[test]
fn test_sidecar_txt_detection_and_upgrade_to_synced() {
    let dir = tempdir().expect("Failed to create tempdir");
    let test_file = dir.path().join("plain_song.wav");
    create_dummy_wav(&test_file);

    // Create a plain text sidecar file plain_song.txt
    let txt_file = dir.path().join("plain_song.txt");
    std::fs::write(&txt_file, "Just plain text lyrics without timing.\nLine two.").unwrap();

    // Read track: should detect as Plain from sidecar .txt
    let track = read_track_metadata(&test_file).expect("Failed to read wav with txt sidecar");
    assert_eq!(track.lyric_state, LyricState::Plain);

    // Now embed Synced LRC lyrics directly into the audio file
    let sample_lrc = "[00:01.00] Synced line one\n[00:05.00] Synced line two";
    embed_lyrics_to_file(&test_file, sample_lrc).expect("Failed to embed synced lyrics");

    // Read track: tag now overrides sidecar txt as Synced!
    let upgraded_track = read_track_metadata(&test_file).expect("Failed to read upgraded wav");
    assert_eq!(upgraded_track.lyric_state, LyricState::Synced);
}
