//! Typed vocabulary shared across endpoints: closed-set enums for request
//! parameters (with an `Other` escape hatch when parsing responses) and the
//! response structures several endpoints return.

pub(crate) mod string_enum;

pub mod enqueue_jobs_response;
pub mod ids;
pub mod image_aspect_ratio;
pub mod image_batch_size;
pub mod image_dimensions;
pub mod image_quality;
pub mod image_resolution;
pub mod image_seed;
pub mod job_media;
pub mod job_params;
pub mod job_set_type;
pub mod job_status;
pub mod nano_banana_aspect_ratio;
pub mod seedream_aspect_ratio;
pub mod thinking_level;
pub mod wallet;
