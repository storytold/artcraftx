use sqlite_identifiers::media_file_token::MediaFileToken;

#[derive(Clone, Debug)]
pub enum VideoListRef {
  MediaFileTokens(Vec<MediaFileToken>),
  Urls(Vec<String>),
}
