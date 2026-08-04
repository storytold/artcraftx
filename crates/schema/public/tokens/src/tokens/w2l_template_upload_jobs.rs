use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// Primary key for the `w2l_template_upload_jobs` table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct W2lTemplateUploadJobToken(pub String);

impl_string_token!(W2lTemplateUploadJobToken);
impl_crockford_generator!(W2lTemplateUploadJobToken, 32usize, LegacyTokenPrefix::W2lTemplateUploadJob, CrockfordLower);
