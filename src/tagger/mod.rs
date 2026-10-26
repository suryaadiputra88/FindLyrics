pub mod reader;
pub mod writer;

pub use reader::{has_lrc_timestamps, read_track_metadata};
pub use writer::embed_lyrics_to_file;
