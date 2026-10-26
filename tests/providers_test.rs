use findlyrics::models::{AudioFormat, LyricState, TrackItem};
use findlyrics::providers::{KugouClient, LrclibClient, MultiProviderClient};
use std::path::PathBuf;

fn create_sample_track(title: &str, artist: &str, duration_secs: f64) -> TrackItem {
    TrackItem {
        path: PathBuf::from("dummy.flac"),
        filename: "dummy.flac".to_string(),
        format: AudioFormat::Flac,
        title: Some(title.to_string()),
        artist: Some(artist.to_string()),
        album: Some("Sample Album".to_string()),
        duration_secs,
        lyric_state: LyricState::None,
        current_lyric_preview: None,
    }
}

#[tokio::test]
async fn test_lrclib_provider() {
    let client = LrclibClient::new();
    let track = create_sample_track("Bohemian Rhapsody", "Queen", 354.0);

    let result = client.fetch_lyrics(&track, 5.0, false).await;
    assert!(result.is_ok());
    if let Ok(Some(payload)) = result {
        assert_eq!(payload.provider, "LRCLIB");
        assert!(payload.best_lyrics().is_some());
    }
}

#[tokio::test]
async fn test_kugou_provider() {
    let client = KugouClient::new();
    let track = create_sample_track("Hotel California", "Eagles", 391.0);

    let result = client.fetch_lyrics(&track, 10.0).await;
    assert!(result.is_ok());
    if let Ok(Some(payload)) = result {
        assert_eq!(payload.provider, "Kugou");
        assert!(payload.is_synced);
        assert!(payload.synced_lyrics.is_some());
    }
}

#[tokio::test]
async fn test_multiprovider_waterfall() {
    let multi = MultiProviderClient::new();
    let track = create_sample_track("Bohemian Rhapsody", "Queen", 354.0);

    let result = multi.fetch_lyrics(&track, 5.0, false).await;
    assert!(result.is_ok());
    let payload = result.unwrap();
    assert!(payload.is_some());
    let p = payload.unwrap();
    assert!(!p.provider.is_empty());
    assert!(p.best_lyrics().is_some());
}
