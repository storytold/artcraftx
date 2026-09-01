//! Reference media for first-party Higgsfield: a source URL (an ArtCraft CDN
//! link, or any public URL) is downloaded and re-uploaded through the
//! session, and the resulting [`MediaInput`] is what generation requests
//! reference. Shared by the image and video Higgsfield providers.

use std::collections::HashMap;

use higgsfield_client::session::higgsfield_session::HiggsfieldSession;
use higgsfield_client::session::upload_media::ReferenceMediaFile;
use higgsfield_client::session::upload_source_guard::check_upload_source_url;
use higgsfield_client::types::media_input::MediaInput;
use higgsfield_client::types::media_mime_type::MediaMimeType;
use log::info;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;
use url_utils::extension::extract_extension_from_url::{extract_extension_from_url_str, ExtractExtensions};

use crate::api::audio_list_ref::AudioListRef;
use crate::api::image_list_ref::ImageListRef;
use crate::api::image_ref::ImageRef;
use crate::api::video_list_ref::VideoListRef;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::client_error::ClientError;
use crate::errors::provider_error::ProviderError;
use crate::utils::download_file::download_file;

/// What a reference file is expected to be. Picks the upload endpoint family
/// and the fallback MIME type when the bytes and URL don't say.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HiggsfieldMediaKind {
  Image,
  Video,
  Audio,
}

impl HiggsfieldMediaKind {
  fn fallback_mime_type(self) -> MediaMimeType {
    match self {
      Self::Image => MediaMimeType::ImageJpeg,
      Self::Video => MediaMimeType::VideoMp4,
      Self::Audio => MediaMimeType::AudioMpeg,
    }
  }

  fn accepts(self, mime_type: &MediaMimeType) -> bool {
    match self {
      Self::Image => mime_type.is_image(),
      Self::Video => mime_type.is_video(),
      Self::Audio => mime_type.is_audio(),
    }
  }
}

/// A list of reference sources before they're resolved to URLs.
pub enum HiggsfieldMediaSources {
  Urls(Vec<String>),
  Tokens(Vec<MediaFileToken>),
}

impl From<ImageListRef> for HiggsfieldMediaSources {
  fn from(list: ImageListRef) -> Self {
    match list {
      ImageListRef::Urls(urls) => Self::Urls(urls),
      ImageListRef::MediaFileTokens(tokens) => Self::Tokens(tokens),
    }
  }
}

impl From<VideoListRef> for HiggsfieldMediaSources {
  fn from(list: VideoListRef) -> Self {
    match list {
      VideoListRef::Urls(urls) => Self::Urls(urls),
      VideoListRef::MediaFileTokens(tokens) => Self::Tokens(tokens),
    }
  }
}

impl From<AudioListRef> for HiggsfieldMediaSources {
  fn from(list: AudioListRef) -> Self {
    match list {
      AudioListRef::Urls(urls) => Self::Urls(urls),
      AudioListRef::MediaFileTokens(tokens) => Self::Tokens(tokens),
    }
  }
}

impl HiggsfieldMediaSources {
  pub fn len(&self) -> usize {
    match self {
      Self::Urls(urls) => urls.len(),
      Self::Tokens(tokens) => tokens.len(),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

/// Upload every source in `list`, in order. `None` / empty lists upload
/// nothing. Media file tokens are resolved through `maybe_map`.
pub async fn upload_media_list(
  session: &HiggsfieldSession,
  list: Option<HiggsfieldMediaSources>,
  kind: HiggsfieldMediaKind,
  ip_check: bool,
  maybe_map: Option<&HashMap<MediaFileToken, String>>,
) -> Result<Vec<MediaInput>, ArtcraftRouterError> {
  let source_urls = match list {
    None => return Ok(Vec::new()),
    Some(HiggsfieldMediaSources::Urls(urls)) => urls,
    Some(HiggsfieldMediaSources::Tokens(tokens)) => resolve_media_tokens(maybe_map, &tokens)?,
  };
  let mut uploaded = Vec::with_capacity(source_urls.len());
  for url in &source_urls {
    uploaded.push(upload_url_to_higgsfield(session, url, kind, ip_check).await?);
  }
  Ok(uploaded)
}

/// Upload a single image reference (a keyframe), if present.
pub async fn upload_image_ref(
  session: &HiggsfieldSession,
  image_ref: Option<ImageRef>,
  ip_check: bool,
  maybe_map: Option<&HashMap<MediaFileToken, String>>,
) -> Result<Option<MediaInput>, ArtcraftRouterError> {
  let source_url = match image_ref {
    None => return Ok(None),
    Some(ImageRef::Url(url)) => url,
    Some(ImageRef::MediaFileToken(token)) => resolve_media_token(maybe_map, &token)?,
  };
  Ok(Some(upload_url_to_higgsfield(session, &source_url, HiggsfieldMediaKind::Image, ip_check).await?))
}

/// Download `source_url` and upload it to Higgsfield as reference media.
/// `ip_check` asks Higgsfield to run (and waits for) its intellectual-property
/// check, which the Seedance video models require on images and clips.
pub async fn upload_url_to_higgsfield(
  session: &HiggsfieldSession,
  source_url: &str,
  kind: HiggsfieldMediaKind,
  ip_check: bool,
) -> Result<MediaInput, ArtcraftRouterError> {
  // Checked again by the client on upload; failing here skips the download.
  check_upload_source_url(source_url)
      .map_err(|err| ArtcraftRouterError::from(ProviderError::Higgsfield(err.into())))?;
  let bytes = download_file(source_url).await?;
  let file = reference_media_file_for_url(source_url, kind, bytes, ip_check);
  info!(
    "Uploading {:?} reference to Higgsfield: {} ({} bytes, {}, ip_check={})",
    kind, source_url, file.bytes.len(), file.mime_type, ip_check,
  );
  session.upload_reference_media(file).await
      .map_err(|err| ArtcraftRouterError::from(ProviderError::Higgsfield(err)))
}

/// Describe downloaded bytes for upload: the MIME type is sniffed from the
/// bytes, then guessed from the URL's extension, then defaulted by `kind`.
pub fn reference_media_file_for_url(source_url: &str, kind: HiggsfieldMediaKind, bytes: Vec<u8>, ip_check: bool) -> ReferenceMediaFile {
  let mime_type = media_mime_type_for(source_url, &bytes, kind);
  let file_name = format!("reference.{}", mime_type.file_extension());
  let file = ReferenceMediaFile::new(file_name, mime_type, bytes).with_source_url(source_url);
  if ip_check { file.with_ip_check() } else { file }
}

/// The best MIME type for a reference: magic bytes first (the URL may have
/// no extension, or lie), then the URL's extension, then the kind's default.
/// A type from the wrong family for `kind` is ignored.
pub fn media_mime_type_for(source_url: &str, bytes: &[u8], kind: HiggsfieldMediaKind) -> MediaMimeType {
  sniff_mime_type(bytes)
      .or_else(|| mime_type_from_url_extension(source_url))
      .filter(|mime_type| kind.accepts(mime_type))
      .unwrap_or_else(|| kind.fallback_mime_type())
}

fn mime_type_from_url_extension(source_url: &str) -> Option<MediaMimeType> {
  let extension = extract_extension_from_url_str(source_url, &ExtractExtensions::All)?;
  MediaMimeType::from_file_name(&format!("file.{}", extension.without_period()))
}

/// Recognise the common image / video / audio containers by their magic
/// bytes. Returns `None` for anything else.
pub fn sniff_mime_type(bytes: &[u8]) -> Option<MediaMimeType> {
  if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
    return Some(MediaMimeType::ImagePng);
  }
  if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
    return Some(MediaMimeType::ImageJpeg);
  }
  if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
    return Some(MediaMimeType::ImageGif);
  }
  if bytes.len() >= 12 && bytes.starts_with(b"RIFF") {
    return match &bytes[8..12] {
      b"WEBP" => Some(MediaMimeType::ImageWebp),
      b"WAVE" => Some(MediaMimeType::AudioWav),
      _ => None,
    };
  }
  if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
    // ISO base media: MP4 / MOV / M4A / HEIC share the box; brand tells.
    return match &bytes[8..12] {
      b"qt  " => Some(MediaMimeType::VideoQuicktime),
      b"M4A " => Some(MediaMimeType::AudioMp4),
      b"heic" | b"heix" | b"mif1" => Some(MediaMimeType::ImageHeic),
      b"avif" => Some(MediaMimeType::ImageAvif),
      _ => Some(MediaMimeType::VideoMp4),
    };
  }
  if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
    // Matroska / WebM. Assume the web flavour; it's what browsers produce.
    return Some(MediaMimeType::VideoWebm);
  }
  if bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0) {
    return Some(MediaMimeType::AudioMpeg);
  }
  if bytes.starts_with(b"OggS") {
    return Some(MediaMimeType::AudioOgg);
  }
  None
}

pub fn resolve_media_token(
  maybe_map: Option<&HashMap<MediaFileToken, String>>,
  token: &MediaFileToken,
) -> Result<String, ArtcraftRouterError> {
  let map = maybe_map.ok_or(ArtcraftRouterError::Client(ClientError::MediaFileToUrlMapNotProvided))?;
  map.get(token).cloned().ok_or_else(|| {
    ArtcraftRouterError::Client(ClientError::MediaFileTokenNotFoundInMap { token: token.clone() })
  })
}

pub fn resolve_media_tokens(
  maybe_map: Option<&HashMap<MediaFileToken, String>>,
  tokens: &[MediaFileToken],
) -> Result<Vec<String>, ArtcraftRouterError> {
  tokens.iter().map(|token| resolve_media_token(maybe_map, token)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
  const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];
  const MP4: &[u8] = b"\0\0\0\x18ftypisom\0\0\x02\0isomiso2";
  const MOV: &[u8] = b"\0\0\0\x14ftypqt  \0\0\0\0qt  ";
  const WAV: &[u8] = b"RIFF\x24\0\0\0WAVEfmt ";
  const WEBP: &[u8] = b"RIFF\x24\0\0\0WEBPVP8 ";
  const MP3_ID3: &[u8] = b"ID3\x04\0\0\0\0\0\0";

  mod sniffing {
    use super::*;

    #[test]
    fn recognises_common_containers() {
      assert_eq!(sniff_mime_type(PNG), Some(MediaMimeType::ImagePng));
      assert_eq!(sniff_mime_type(JPEG), Some(MediaMimeType::ImageJpeg));
      assert_eq!(sniff_mime_type(WEBP), Some(MediaMimeType::ImageWebp));
      assert_eq!(sniff_mime_type(MP4), Some(MediaMimeType::VideoMp4));
      assert_eq!(sniff_mime_type(MOV), Some(MediaMimeType::VideoQuicktime));
      assert_eq!(sniff_mime_type(WAV), Some(MediaMimeType::AudioWav));
      assert_eq!(sniff_mime_type(MP3_ID3), Some(MediaMimeType::AudioMpeg));
      assert_eq!(sniff_mime_type(&[0x1A, 0x45, 0xDF, 0xA3, 0x01]), Some(MediaMimeType::VideoWebm));
    }

    #[test]
    fn unknown_bytes_are_none() {
      assert_eq!(sniff_mime_type(b"hello world"), None);
      assert_eq!(sniff_mime_type(b""), None);
      assert_eq!(sniff_mime_type(b"RIFF\0\0\0\0AVI "), None);
    }
  }

  mod mime_resolution {
    use super::*;

    #[test]
    fn bytes_win_over_extension() {
      // A PNG served from a ".jpg" URL is still a PNG.
      assert_eq!(media_mime_type_for("https://cdn.example.com/a.jpg", PNG, HiggsfieldMediaKind::Image), MediaMimeType::ImagePng);
    }

    #[test]
    fn extension_is_used_when_bytes_are_unrecognised() {
      assert_eq!(media_mime_type_for("https://cdn.example.com/a.webp?x=1", b"????", HiggsfieldMediaKind::Image), MediaMimeType::ImageWebp);
      assert_eq!(media_mime_type_for("https://cdn.example.com/clip.mov", b"????", HiggsfieldMediaKind::Video), MediaMimeType::VideoQuicktime);
      assert_eq!(media_mime_type_for("https://cdn.example.com/song.mp3", b"????", HiggsfieldMediaKind::Audio), MediaMimeType::AudioMpeg);
    }

    #[test]
    fn falls_back_by_kind() {
      assert_eq!(media_mime_type_for("https://cdn.example.com/media/abc", b"????", HiggsfieldMediaKind::Image), MediaMimeType::ImageJpeg);
      assert_eq!(media_mime_type_for("https://cdn.example.com/media/abc", b"????", HiggsfieldMediaKind::Video), MediaMimeType::VideoMp4);
      assert_eq!(media_mime_type_for("https://cdn.example.com/media/abc", b"????", HiggsfieldMediaKind::Audio), MediaMimeType::AudioMpeg);
    }

    #[test]
    fn wrong_family_for_kind_is_ignored() {
      // An image URL handed in as a video reference: don't claim it's a PNG video.
      assert_eq!(media_mime_type_for("https://cdn.example.com/a.png", PNG, HiggsfieldMediaKind::Video), MediaMimeType::VideoMp4);
    }

    #[test]
    fn reference_file_carries_extension_and_ip_check() {
      let file = reference_media_file_for_url("https://cdn.example.com/x", HiggsfieldMediaKind::Video, MP4.to_vec(), true);
      assert_eq!(file.file_name, "reference.mp4");
      assert_eq!(file.mime_type, MediaMimeType::VideoMp4);
      assert!(file.force_ip_check);
      let file = reference_media_file_for_url("https://cdn.example.com/x.png", HiggsfieldMediaKind::Image, PNG.to_vec(), false);
      assert_eq!(file.file_name, "reference.png");
      assert!(!file.force_ip_check);
      assert_eq!(file.maybe_source_url.as_deref(), Some("https://cdn.example.com/x.png"));
    }
  }

  mod token_resolution {
    use super::*;

    #[test]
    fn tokens_resolve_through_the_map() {
      let token = MediaFileToken::new_from_str("m_abc");
      let mut map = HashMap::new();
      map.insert(token.clone(), "https://cdn.example.com/abc.png".to_string());
      assert_eq!(resolve_media_token(Some(&map), &token).unwrap(), "https://cdn.example.com/abc.png");
      assert!(matches!(
        resolve_media_token(None, &token),
        Err(ArtcraftRouterError::Client(ClientError::MediaFileToUrlMapNotProvided)),
      ));
      assert!(matches!(
        resolve_media_token(Some(&map), &MediaFileToken::new_from_str("m_missing")),
        Err(ArtcraftRouterError::Client(ClientError::MediaFileTokenNotFoundInMap { .. })),
      ));
    }

    #[test]
    fn source_lists_convert_from_every_ref_type() {
      let images: HiggsfieldMediaSources = ImageListRef::Urls(vec!["a".into(), "b".into()]).into();
      assert_eq!(images.len(), 2);
      let videos: HiggsfieldMediaSources = VideoListRef::MediaFileTokens(vec![MediaFileToken::new_from_str("m_1")]).into();
      assert_eq!(videos.len(), 1);
      let audio: HiggsfieldMediaSources = AudioListRef::Urls(vec![]).into();
      assert!(audio.is_empty());
    }
  }
}
