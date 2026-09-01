use std::path::Path;
use std::time::Duration;

use errors::AnyhowResult;
use log::{error, info};
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Pool, Sqlite, SqlitePool};

/// How long a writer waits for the lock before erroring. Thumbnail
/// generation writes in bursts; without this SQLite returns SQLITE_BUSY
/// immediately.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct LocalFilesDbConnection {
  pool: Pool<Sqlite>,
}

impl LocalFilesDbConnection {
  /// Open (creating if missing) and migrate the database.
  ///
  /// On migration failure the file is deleted and recreated — acceptable
  /// while every row is derivable from the filesystem (hashes and thumbnail
  /// flags regenerate on demand). Revisit before storing anything
  /// irreplaceable here (e.g. the upload-dedup ledger).
  pub async fn connect_and_migrate<P: AsRef<Path>>(database_file: P) -> AnyhowResult<Self> {
    match run_migrations(&database_file).await {
      Ok(pool) => return Ok(Self { pool }),
      Err(err) => {
        error!("Error running local_files SQLite migrations: {:?}", err);
      }
    }

    info!("Deleting and recreating local_files SQLite database at {:?}", database_file.as_ref());

    if let Err(err) = std::fs::remove_file(&database_file) {
      error!("Error deleting local_files SQLite database file: {:?}", err);
    }

    let pool = run_migrations(database_file).await?;

    Ok(Self { pool })
  }

  pub fn get_pool(&self) -> &Pool<Sqlite> {
    &self.pool
  }
}

async fn run_migrations<P: AsRef<Path>>(database_file: P) -> Result<SqlitePool, MigrateError> {
  let connection_options = SqliteConnectOptions::new()
      .filename(database_file)
      .create_if_missing(true)
      .journal_mode(SqliteJournalMode::Wal)
      .synchronous(SqliteSynchronous::Normal)
      .busy_timeout(BUSY_TIMEOUT);

  let pool = SqlitePool::connect_with(connection_options).await?;

  // Additive migrations against a stable filename; sqlx tracks which have
  // run. The SQL is compiled into the binary.
  sqlx::migrate!().run(&pool).await?;

  Ok(pool)
}
