pub mod kugou;
pub mod lrclib;
pub mod lyricsovh;
pub mod multi;

pub use kugou::KugouClient;
pub use lrclib::LrclibClient;
pub use lyricsovh::LyricsOvhClient;
pub use multi::MultiProviderClient;
