#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use thiserror::Error;
use tokio_postgres::Client;

pub const DEFAULT_MIGRATIONS_DIR: &str = "db/migrations";
pub const MAX_MIGRATIONS: usize = 4_096;
pub const MAX_MIGRATION_BYTES: u64 = 8 * 1024 * 1024;
pub const DEFAULT_STATEMENT_TIMEOUT_MS: u64 = 300_000;
const ADVISORY_LOCK_KEY: &str = "solana-tax-protocol/schema-migrations/v1";

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("migration directory is not accessible: {0}")]
    Directory(#[from] std::io::Error),
    #[error("migration filename is invalid: {0}")]
    InvalidFilename(String),
    #[error("migration version is duplicated: {0}")]
    DuplicateVersion(u32),
    #[error("migration file is too large: {0}")]
    FileTooLarge(String),
    #[error("migration SQL is unreadable: {0}")]
    ReadSql(String),
    #[error("migration database operation failed: {0}")]
    Database(#[from] tokio_postgres::Error),
    #[error(
        "applied migration checksum drift detected for {migration_id}; applied migrations are immutable — restore the matching artifact or add a forward migration"
    )]
    ChecksumDrift { migration_id: String },
    #[error("migration statement timeout must be between 1000 and 900000 milliseconds")]
    InvalidTimeout,
    #[error("migration transaction markers must include both BEGIN; and COMMIT;")]
    InvalidTransactionBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationFile {
    pub migration_id: String,
    pub version: u32,
    pub path: PathBuf,
    pub checksum: [u8; 32],
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunSummary {
    pub applied: usize,
    pub skipped: usize,
    pub seeded: usize,
}

/// Discovers and validates the immutable migration source set before any
/// database connection is mutated.
pub fn discover_migrations(directory: &Path) -> Result<Vec<MigrationFile>, MigrationError> {
    let mut paths = fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("sql"))
        .collect::<Vec<_>>();
    paths.sort();
    if paths.len() > MAX_MIGRATIONS {
        return Err(MigrationError::InvalidFilename(
            "migration directory exceeds the file bound".to_string(),
        ));
    }

    let mut migrations = Vec::with_capacity(paths.len());
    let mut previous_version = None;
    for path in paths {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| MigrationError::InvalidFilename(path.display().to_string()))?;
        let (version, migration_id) = parse_filename(file_name)?;
        if previous_version == Some(version) {
            return Err(MigrationError::DuplicateVersion(version));
        }
        if previous_version.is_some_and(|previous| version < previous) {
            return Err(MigrationError::InvalidFilename(format!(
                "migration versions are not ordered: {file_name}"
            )));
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_MIGRATION_BYTES {
            return Err(MigrationError::FileTooLarge(file_name.to_string()));
        }
        let bytes = fs::read(&path)
            .map_err(|error| MigrationError::ReadSql(format!("{}: {error}", path.display())))?;
        let checksum = Sha256::digest(&bytes).into();
        migrations.push(MigrationFile {
            migration_id,
            version,
            path,
            checksum,
            bytes,
        });
        previous_version = Some(version);
    }
    Ok(migrations)
}

fn parse_filename(file_name: &str) -> Result<(u32, String), MigrationError> {
    if !file_name.ends_with(".sql") || file_name.len() < 10 {
        return Err(MigrationError::InvalidFilename(file_name.to_string()));
    }
    let stem = &file_name[..file_name.len() - 4];
    let (version_text, name) = stem
        .split_once('_')
        .ok_or_else(|| MigrationError::InvalidFilename(file_name.to_string()))?;
    if version_text.len() != 4
        || name.is_empty()
        || !version_text.bytes().all(|byte| byte.is_ascii_digit())
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(MigrationError::InvalidFilename(file_name.to_string()));
    }
    let version = version_text
        .parse::<u32>()
        .map_err(|_| MigrationError::InvalidFilename(file_name.to_string()))?;
    Ok((version, stem.to_string()))
}

pub async fn apply_migrations(
    client: &Client,
    migrations: &[MigrationFile],
    statement_timeout_ms: u64,
) -> Result<RunSummary, MigrationError> {
    apply_migrations_with_seeds(client, migrations, &[], statement_timeout_ms).await
}

pub async fn apply_migrations_with_seeds(
    client: &Client,
    migrations: &[MigrationFile],
    seeds: &[MigrationFile],
    statement_timeout_ms: u64,
) -> Result<RunSummary, MigrationError> {
    if !(1_000..=900_000).contains(&statement_timeout_ms) {
        return Err(MigrationError::InvalidTimeout);
    }
    client
        .batch_execute(&format!(
            "SET lock_timeout = '5000ms'; SET statement_timeout = '{statement_timeout_ms}ms'; SET idle_in_transaction_session_timeout = '60000ms';"
        ))
        .await?;
    client
        .query_one(
            "SELECT pg_advisory_lock(hashtextextended($1, 0))",
            &[&ADVISORY_LOCK_KEY],
        )
        .await?;

    let result = apply_locked(client, migrations, seeds).await;
    let unlock_result = client
        .query_one(
            "SELECT pg_advisory_unlock(hashtextextended($1, 0))",
            &[&ADVISORY_LOCK_KEY],
        )
        .await;
    match (result, unlock_result) {
        (Err(error), _) => Err(error),
        (Ok(summary), Ok(_)) => Ok(summary),
        (Ok(_), Err(error)) => Err(MigrationError::Database(error)),
    }
}

async fn apply_locked(
    client: &Client,
    migrations: &[MigrationFile],
    seeds: &[MigrationFile],
) -> Result<RunSummary, MigrationError> {
    client
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (\
                migration_id text PRIMARY KEY CHECK (length(migration_id) BETWEEN 10 AND 128),\
                version integer NOT NULL UNIQUE CHECK (version >= 0 AND version <= 2147483647),\
                checksum bytea NOT NULL CHECK (octet_length(checksum) = 32),\
                applied_at timestamptz NOT NULL DEFAULT clock_timestamp(),\
                execution_ms bigint NOT NULL CHECK (execution_ms >= 0)\
            );",
        )
        .await?;

    let mut applied = 0;
    let mut skipped = 0;
    for migration in migrations {
        let existing = client
            .query_opt(
                "SELECT checksum FROM schema_migrations WHERE migration_id = $1",
                &[&migration.migration_id],
            )
            .await?;
        if let Some(row) = existing {
            let checksum: Vec<u8> = row.get(0);
            if checksum.as_slice() != migration.checksum {
                return Err(MigrationError::ChecksumDrift {
                    migration_id: migration.migration_id.clone(),
                });
            }
            skipped += 1;
            continue;
        }

        let started = Instant::now();
        let sql = std::str::from_utf8(&migration.bytes).map_err(|_| {
            MigrationError::ReadSql(format!("{} is not valid UTF-8", migration.path.display()))
        })?;
        // Existing migrations may own their explicit transaction boundary. New
        // migrations without one are wrapped here so a failed file cannot
        // leave a partial schema change behind before its ledger row exists.
        let explicit_transaction = transaction_markers(sql)?;
        if !explicit_transaction {
            client.batch_execute("BEGIN;").await?;
        }
        if let Err(error) = client.batch_execute(sql).await {
            let _ = client.batch_execute("ROLLBACK;").await;
            return Err(MigrationError::Database(error));
        }
        let execution_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
        if let Err(error) = client
            .execute(
                "INSERT INTO schema_migrations (migration_id, version, checksum, execution_ms) VALUES ($1, $2, $3, $4)",
                &[&migration.migration_id, &(migration.version as i32), &&migration.checksum[..], &execution_ms],
            )
            .await
        {
            let _ = client.batch_execute("ROLLBACK;").await;
            return Err(MigrationError::Database(error));
        }
        if !explicit_transaction {
            client.batch_execute("COMMIT;").await?;
        }
        applied += 1;
    }
    let mut seeded = 0;
    if !seeds.is_empty() {
        client.batch_execute("BEGIN;").await?;
    }
    for seed in seeds {
        let sql = std::str::from_utf8(&seed.bytes).map_err(|_| {
            MigrationError::ReadSql(format!("{} is not valid UTF-8", seed.path.display()))
        });
        let sql = match sql {
            Ok(sql) => sql,
            Err(error) => {
                let _ = client.batch_execute("ROLLBACK;").await;
                return Err(error);
            }
        };
        if let Err(error) = client.batch_execute(sql).await {
            let _ = client.batch_execute("ROLLBACK;").await;
            return Err(MigrationError::Database(error));
        }
        seeded += 1;
    }
    if !seeds.is_empty() {
        client.batch_execute("COMMIT;").await?;
    }
    Ok(RunSummary {
        applied,
        skipped,
        seeded,
    })
}

fn transaction_markers(sql: &str) -> Result<bool, MigrationError> {
    let has_begin = sql.lines().any(|line| line.trim() == "BEGIN;");
    let has_commit = sql.lines().any(|line| line.trim() == "COMMIT;");
    if has_begin != has_commit {
        return Err(MigrationError::InvalidTransactionBoundary);
    }
    Ok(has_begin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{create_dir, remove_dir_all, write},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_directory() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let suffix = NEXT.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "solana-tax-protocol-migrations-{}-{timestamp}-{suffix}",
            std::process::id()
        ));
        create_dir(&directory).expect("create migration test directory");
        directory
    }

    #[test]
    fn discovers_ordered_files_and_stable_checksums() {
        let directory = temp_directory();
        write(directory.join("0002_second.sql"), "SELECT 2;").expect("write second");
        write(directory.join("0001_first.sql"), "SELECT 1;").expect("write first");
        write(directory.join("README.txt"), "ignored").expect("write note");

        let migrations = discover_migrations(&directory).expect("discover migrations");
        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].migration_id, "0001_first");
        assert_eq!(migrations[1].version, 2);
        assert_eq!(
            migrations[0].checksum,
            <[u8; 32]>::from(Sha256::digest(b"SELECT 1;"))
        );
        remove_dir_all(directory).expect("remove migration test directory");
    }

    #[test]
    fn checksum_drift_diagnostic_preserves_forward_only_recovery() {
        let message = MigrationError::ChecksumDrift {
            migration_id: "0060_revenue_streaming_bounded_context".to_string(),
        }
        .to_string();

        assert!(message.contains("applied migrations are immutable"));
        assert!(message.contains("restore the matching artifact"));
        assert!(message.contains("add a forward migration"));
    }

    #[test]
    fn duplicate_numeric_versions_fail_closed() {
        let directory = temp_directory();
        write(directory.join("0001_first.sql"), "SELECT 1;").expect("write first");
        write(directory.join("0001_second.sql"), "SELECT 2;").expect("write second");

        assert!(matches!(
            discover_migrations(&directory),
            Err(MigrationError::DuplicateVersion(1))
        ));
        remove_dir_all(directory).expect("remove migration test directory");
    }

    #[test]
    fn malformed_names_and_oversized_files_fail_closed() {
        let directory = temp_directory();
        write(directory.join("migration.sql"), "SELECT 1;").expect("write malformed");
        assert!(matches!(
            discover_migrations(&directory),
            Err(MigrationError::InvalidFilename(_))
        ));
        remove_dir_all(directory).expect("remove migration test directory");

        let directory = temp_directory();
        let oversized = vec![b'x'; (MAX_MIGRATION_BYTES + 1) as usize];
        write(directory.join("0001_large.sql"), oversized).expect("write large");
        assert!(matches!(
            discover_migrations(&directory),
            Err(MigrationError::FileTooLarge(_))
        ));
        remove_dir_all(directory).expect("remove migration test directory");
    }

    #[test]
    fn transaction_markers_are_balanced_and_detected() {
        assert!(transaction_markers("BEGIN;\nSELECT 1;\nCOMMIT;").expect("markers"));
        assert!(!transaction_markers("SELECT 1;").expect("implicit transaction"));
        assert!(matches!(
            transaction_markers("BEGIN;\nSELECT 1;"),
            Err(MigrationError::InvalidTransactionBoundary)
        ));
    }
}
