//! Hard ceilings of the deck knowledge bundle; core owns them, configuration may only lower them.

/// Most files in a bundle.
pub const MAX_FILES: usize = 1024;
/// Most bytes in one file.
pub const MAX_FILE_BYTES: usize = 1 << 20;
/// Most bytes in the whole bundle.
pub const MAX_TOTAL_BYTES: usize = 16 << 20;
/// How far past each limit above a replica is still read: merged peers may pass a limit and
/// keep every file (R10), only a forged replica passes `READ_FACTOR` times one. Entries past
/// that read ceiling are dropped and counted, never materialized.
pub const READ_FACTOR: usize = 4;
/// Most bytes in a frontmatter block.
pub const MAX_FRONTMATTER_BYTES: usize = 16 << 10;
/// Deepest nesting of a frontmatter YAML value.
pub const MAX_YAML_DEPTH: usize = 16;
/// Most bytes in a knowledge path.
pub const PATH_MAX_BYTES: usize = 255;
/// Most segments in a knowledge path.
pub const PATH_MAX_SEGMENTS: usize = 8;
