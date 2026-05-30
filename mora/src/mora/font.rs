/// Default monospace font embedded at compile time.
///
/// JetBrainsMonoNerdFont-Regular.ttf — good Unicode coverage, ligatures,
/// and Nerd Font icon support.
pub const DEFAULT_FONT: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/JetBrainsMonoNerdFont-Regular.ttf"));
