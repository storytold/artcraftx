//! First-party (cookie-session) Grok Imagine image generation.
//!
//! Drives the user's own grok.com "imagine" websocket via captured cookies
//! (see `RouterGrokClient`), returning the finished image URLs directly (there
//! is no job id to poll). Fast vs quality ("pro") is a flag on the request.
pub mod grok_imagine_image;
