use sqlite_identifiers::ids::media_file_token::MediaFileToken;

#[derive(Clone, Debug)]
pub enum ImageRef {
  MediaFileToken(MediaFileToken),
  Url(String),
}
