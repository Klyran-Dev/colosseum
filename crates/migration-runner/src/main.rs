#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use migration_runner::{
    DEFAULT_MIGRATIONS_DIR, DEFAULT_STATEMENT_TIMEOUT_MS, apply_migrations_with_seeds,
    discover_migrations,
};
use std::{env, error::Error, future::Future, process::ExitCode, time::Instant};
use tokio_postgres::NoTls;

type RunnerError = Box<dyn Error + Send + Sync>;

fn configured_seed_dir(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(summary) => {
            println!(
                "migration-runner applied={} skipped={} seeded={}",
                summary.applied, summary.skipped, summary.seeded
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("migration-runner failed closed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<migration_runner::RunSummary, RunnerError> {
    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set for migration execution")?;
    let migrations_dir =
        env::var("MIGRATIONS_DIR").unwrap_or_else(|_| DEFAULT_MIGRATIONS_DIR.to_string());
    let timeout = env::var("MIGRATION_STATEMENT_TIMEOUT_MS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(DEFAULT_STATEMENT_TIMEOUT_MS);
    let migrations = discover_migrations(std::path::Path::new(&migrations_dir))?;
    let seed_directory = env::var("SEED_DIR").ok();
    let seeds = configured_seed_dir(seed_directory.as_deref())
        .map(|directory| discover_migrations(std::path::Path::new(directory)))
        .transpose()?
        .unwrap_or_default();
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    let started = Instant::now();
    let migration = async {
        apply_migrations_with_seeds(&client, &migrations, &seeds, timeout)
            .await
            .map_err(|error| Box::new(error) as RunnerError)
    };
    let connection = async {
        connection
            .await
            .map_err(|error| Box::new(error) as RunnerError)
    };
    let summary = run_with_connection(migration, connection).await?;
    eprintln!(
        "migration-runner completed in {}ms",
        started.elapsed().as_millis()
    );
    Ok(summary)
}

async fn run_with_connection<M, C, T>(migration: M, connection: C) -> Result<T, RunnerError>
where
    M: Future<Output = Result<T, RunnerError>>,
    C: Future<Output = Result<(), RunnerError>>,
{
    tokio::pin!(migration);
    tokio::select! {
        result = &mut migration => result,
        result = connection => match result {
            Ok(()) => Err("database connection ended before migration completed".into()),
            Err(error) => Err(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{configured_seed_dir, run_with_connection};
    use std::{error::Error, time::Duration};

    type BoxError = Box<dyn Error + Send + Sync>;

    #[test]
    fn blank_seed_directory_is_disabled() {
        assert_eq!(configured_seed_dir(None), None);
        assert_eq!(configured_seed_dir(Some("")), None);
        assert_eq!(configured_seed_dir(Some("   ")), None);
        assert_eq!(configured_seed_dir(Some("/seeds")), Some("/seeds"));
    }

    #[tokio::test]
    async fn connection_failure_wins_and_fails_the_runner_closed() {
        let result = run_with_connection(
            async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok::<_, BoxError>(7_u8)
            },
            async {
                Err::<(), BoxError>(std::io::Error::other("database connection failed").into())
            },
        )
        .await;

        assert_eq!(
            result
                .expect_err("connection failure must fail the runner")
                .to_string(),
            "database connection failed"
        );
    }

    #[tokio::test]
    async fn completed_migration_does_not_wait_for_the_connection_driver() {
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            run_with_connection(async { Ok::<_, BoxError>(7_u8) }, async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok::<(), BoxError>(())
            }),
        )
        .await
        .expect("migration supervisor should return when migration completes")
        .expect("migration should succeed");

        assert_eq!(result, 7);
    }
}
