//! Integration tests against a throwaway migrated database in a tempdir.

use local_files_database::connection::LocalFilesDbConnection;
use local_files_database::queries::get_local_file_by_path::get_local_file_by_path;
use local_files_database::queries::get_local_files_by_hash::get_local_files_by_hash;
use local_files_database::queries::set_thumbnail_flags::set_thumbnail_flags;
use local_files_database::queries::upsert_local_file::{upsert_local_file, UpsertLocalFileArgs};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

async fn test_db(dir: &tempfile::TempDir) -> LocalFilesDbConnection {
  LocalFilesDbConnection::connect_and_migrate(dir.path().join("local_files.sqlite"))
      .await
      .expect("connect and migrate")
}

async fn upsert(db: &LocalFilesDbConnection, path: &str, size: i64, mtime: i64, hash: &str) {
  upsert_local_file(UpsertLocalFileArgs {
    db,
    file_path: path,
    maybe_file_type: Some("mp4"),
    file_size_bytes: size,
    file_mtime_ms: mtime,
    file_hash_blake3: hash,
  })
      .await
      .expect("upsert");
}

#[tokio::test]
async fn insert_read_and_stat_matching() {
  let dir = tempfile::tempdir().unwrap();
  let db = test_db(&dir).await;

  assert!(get_local_file_by_path(&db, "/a/video.mp4").await.unwrap().is_none());

  upsert(&db, "/a/video.mp4", 1000, 111, HASH_A).await;
  let record = get_local_file_by_path(&db, "/a/video.mp4").await.unwrap().unwrap();
  assert_eq!(record.file_hash_blake3, HASH_A);
  assert_eq!(record.file_type.as_deref(), Some("mp4"));
  assert!(!record.has_thumbnail);
  assert!(record.matches_stat(1000, 111));
  assert!(!record.matches_stat(1000, 222));
  assert!(!record.matches_stat(999, 111));
}

#[tokio::test]
async fn thumbnail_flags_survive_same_hash_upsert_but_reset_on_new_hash() {
  let dir = tempfile::tempdir().unwrap();
  let db = test_db(&dir).await;

  upsert(&db, "/a/video.mp4", 1000, 111, HASH_A).await;
  assert!(set_thumbnail_flags(&db, "/a/video.mp4", true, true).await.unwrap());
  assert!(!set_thumbnail_flags(&db, "/missing", true, true).await.unwrap());

  // Same content re-upserted (e.g. mtime refreshed): flags survive.
  upsert(&db, "/a/video.mp4", 1000, 222, HASH_A).await;
  let record = get_local_file_by_path(&db, "/a/video.mp4").await.unwrap().unwrap();
  assert!(record.has_thumbnail);
  assert!(record.has_animated_preview);
  assert_eq!(record.file_mtime_ms, 222);

  // File overwritten in place with new content: flags reset.
  upsert(&db, "/a/video.mp4", 2000, 333, HASH_B).await;
  let record = get_local_file_by_path(&db, "/a/video.mp4").await.unwrap().unwrap();
  assert!(!record.has_thumbnail);
  assert!(!record.has_animated_preview);
  assert_eq!(record.file_hash_blake3, HASH_B);
}

#[tokio::test]
async fn hash_lookup_finds_duplicate_paths() {
  let dir = tempfile::tempdir().unwrap();
  let db = test_db(&dir).await;

  upsert(&db, "/a/video.mp4", 1000, 111, HASH_A).await;
  upsert(&db, "/b/copy.mp4", 1000, 999, HASH_A).await;
  upsert(&db, "/c/other.mp4", 500, 111, HASH_B).await;

  let matches = get_local_files_by_hash(&db, HASH_A).await.unwrap();
  assert_eq!(matches.len(), 2);
  assert_eq!(matches[0].file_path, "/a/video.mp4");
  assert_eq!(matches[1].file_path, "/b/copy.mp4");
}
