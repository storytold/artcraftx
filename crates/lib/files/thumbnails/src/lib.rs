//! thumbnails
//!
//! Pure-Rust thumbnail generation for local media files: static JPEG
//! thumbnails for images and videos, plus a ~1-second animated WebP preview
//! for videos. Video support covers H.264 in MP4 — what every current
//! provider emits — via vendored decoders (no ffmpeg, no runtime deps on
//! the customer machine). Anything else returns a typed "unsupported"
//! error for the caller to placeholder.

// Never allow these
#![forbid(private_bounds)]
#![forbid(private_interfaces)]
#![forbid(unused_must_use)]

// Okay to toggle
#![forbid(unreachable_patterns)]
#![forbid(unused_imports)]
#![forbid(unused_mut)]
#![forbid(unused_variables)]

// Always allow
#![allow(dead_code)]

pub mod error;
pub mod image_thumbnail;
pub mod mp4_frames;
pub mod video_thumbnail;

pub use error::ThumbnailError;
pub use image_thumbnail::generate_image_thumbnail;
pub use video_thumbnail::generate_video_thumbnails;

/// Longest edge of static thumbnails (matches the frontend's
/// `THUMBNAIL_SIZES.LARGE`).
pub const STATIC_THUMBNAIL_MAX_DIMENSION: u32 = 512;

/// Longest edge of animated previews (kept small; these hold ~24 frames).
pub const ANIMATED_PREVIEW_MAX_DIMENSION: u32 = 320;

/// How much leading video the animated preview covers.
pub const ANIMATED_PREVIEW_MAX_DURATION_MS: u32 = 1_100;

/// Frame cap for the animated preview (~1s at 24fps, plus headroom).
pub const ANIMATED_PREVIEW_MAX_FRAMES: usize = 30;
