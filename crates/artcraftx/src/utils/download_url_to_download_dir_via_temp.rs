use crate::error::artcraftx_error::ArtcraftXError;
use crate::state::app_preferences::app_preferences::AppPreferences;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::utils::download_url_to_user_download_dir::download_file_name_from_url;
use anyhow::anyhow;
use log::info;
use std::io::Write;
use std::path::PathBuf;
use url::Url;

/// Download a file to the app's temp directory, then move it into the user's
/// configured download directory. Downloading to temp first means partially
/// downloaded files never land in the download directory.
///
/// Returns the final path. If a file with the same name already exists in the
/// download directory, nothing is downloaded and
/// [`ArtcraftXError::CannotDownloadFilePathAlreadyExists`] is returned.
pub async fn download_url_to_download_dir_via_temp(
  url: &Url,
  app_data_root: &AppDataRoot,
  app_prefs: &AppPreferences,
) -> Result<PathBuf, ArtcraftXError> {
  let url_file_name = download_file_name_from_url(url)?;

  let download_directory = app_prefs
      .preferred_download_directory
      .download_directory(app_data_root);

  let destination = {
    let mut destination = download_directory.clone();
    destination.push(&url_file_name);
    destination
  };

  if destination == download_directory {
    return Err(ArtcraftXError::AnyhowError(anyhow!("Download filename resolved to directory: {:?}", destination)));
  }

  if destination.exists() {
    if destination.is_dir() {
      return Err(ArtcraftXError::AnyhowError(anyhow!("Download path exists and resolved to directory: {:?}", destination)));
    }
    return Err(ArtcraftXError::CannotDownloadFilePathAlreadyExists { path: destination });
  }

  // Download to the temp directory first.
  let extension = url_file_name.rsplit('.').next().unwrap_or("bin");

  let response = reqwest::get(url.clone()).await?;
  let response_bytes = response.bytes().await?;

  let mut temp_file = app_data_root
      .temp_dir()
      .new_named_temp_file_with_extension(extension)?;
  temp_file.write_all(&response_bytes)?;
  temp_file.flush()?;

  info!("Downloaded {:?} to temp; moving to {:?}", url.as_str(), destination);

  // Move into place. `persist` is a rename, which fails across filesystems
  // (the user's download directory may be on another volume) — fall back to
  // copy + cleanup.
  match temp_file.persist_noclobber(&destination) {
    Ok(_file) => {}
    Err(persist_error) => {
      let temp_file = persist_error.file;
      std::fs::copy(temp_file.path(), &destination)?;
      // NB: The temp file cleans itself up on drop.
    }
  }

  Ok(destination)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Live: downloads a real CDN file through the temp-then-move path.
  #[tokio::test]
  #[ignore] // Live: hits the CDN (read-only) and writes to the default download dir.
  async fn live_download_to_download_dir() {
    let app_data_root = AppDataRoot::create_default().expect("app data root");
    let app_prefs = AppPreferences::default();

    // A known completed generation output.
    let url = Url::parse("https://cdn-2.fakeyou.com/media/j/q/z/s/x/jqzsxxd8r20bnx42jw2dhdj6tq81q9xf/artcraft_jqzsxxd8r20bnx42jw2dhdj6tq81q9xf.png").unwrap();

    let path = download_url_to_download_dir_via_temp(&url, &app_data_root, &app_prefs)
        .await
        .expect("download should succeed");

    println!("[live] downloaded to {:?}", path);
    assert!(path.exists());
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    assert!(path.starts_with(app_prefs.preferred_download_directory.download_directory(&app_data_root)));

    // Second attempt must refuse to clobber.
    let second = download_url_to_download_dir_via_temp(&url, &app_data_root, &app_prefs).await;
    assert!(matches!(second, Err(ArtcraftXError::CannotDownloadFilePathAlreadyExists { .. })));

    std::fs::remove_file(&path).expect("cleanup");
  }
}
