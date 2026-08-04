use sqlite_identifiers::media_file_token::MediaFileToken;

#[derive(Clone, Debug)]
pub enum VideoRef {
  MediaFileToken(MediaFileToken),
  Url(String),
}
