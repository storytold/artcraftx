use std::path::Path;

#[cfg(unix)]
use std::fs::DirBuilder;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

pub trait DataSubdir  : Sized {
  const DIRECTORY_NAME: &'static str;

  /// Create the directory so only its owner can open it (0700 on Unix).
  /// For directories that hold secrets.
  const OWNER_ONLY: bool = false;

  fn new_from<P: AsRef<Path>>(dir: P) -> Self;

  fn path(&self) -> &Path;

  fn get_or_create_in_root_dir<P: AsRef<Path>>(root_dir: P) -> anyhow::Result<Self> {
    Self::get_or_create_dir(root_dir.as_ref().join(Self::DIRECTORY_NAME))
  }

  fn get_or_create_dir<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
    let mut dir = dir.as_ref().to_path_buf();

    match dir.canonicalize() {
      Ok(d) => dir = d,
      Err(err) => {
        println!("Error canonicalizing {:?}: {}", dir, err);
      }
    }

    if !dir.exists() {
      println!("Creating directory {:?}", dir);
      create_dir(&dir, Self::OWNER_ONLY)?;
    }

    Ok(Self::new_from(dir))
  }
}

fn create_dir(dir: &Path, owner_only: bool) -> std::io::Result<()> {
  if owner_only {
    return create_owner_only_dir(dir);
  }
  std::fs::create_dir(dir)
}

/// The mode is set when the directory is created, so it is never briefly
/// open to other users.
#[cfg(unix)]
fn create_owner_only_dir(dir: &Path) -> std::io::Result<()> {
  DirBuilder::new().mode(0o700).create(dir)
}

#[cfg(not(unix))]
fn create_owner_only_dir(dir: &Path) -> std::io::Result<()> {
  std::fs::create_dir(dir)
}
