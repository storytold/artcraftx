use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for uploaded_videos.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct UploadedVideoToken(pub String);

impl_string_token!(UploadedVideoToken);
impl_crockford_generator!(UploadedVideoToken, 32usize, TokenPrefix::UploadedVideo, CrockfordLower);
