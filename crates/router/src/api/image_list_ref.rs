use sqlite_identifiers::ids::media_file_token::MediaFileToken;

#[derive(Clone, Debug)]
pub enum ImageListRef {
  MediaFileTokens(Vec<MediaFileToken>),
  Urls(Vec<String>),
}
