use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for uploaded_video_notes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct UploadVideoNoteToken(pub String);

impl_string_token!(UploadVideoNoteToken);
impl_crockford_generator!(UploadVideoNoteToken, 32usize, TokenPrefix::UploadedVideoNote, CrockfordLower);
