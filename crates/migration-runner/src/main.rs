#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use migration_runner::{
    DEFAULT_MIGRATIONS_DIR, DEFAULT_STATEMENT_TIMEOUT_MS, apply_migrations_with_seeds,
    discover_migrations,
};
use std::{env, process::ExitCode, time::Instant};
use tokio_postgres::NoTls;

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

async fn run() -> Result<migration_runner::RunSummary, Box<dyn std::error::Error>> {
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
    let seeds = env::var("SEED_DIR")
        .ok()
        .map(|directory| discover_migrations(std::path::Path::new(&directory)))
        .transpose()?
        .unwrap_or_default();
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("migration database connection failed: {error}");
        }
    });
    let started = Instant::now();
    let summary = apply_migrations_with_seeds(&client, &migrations, &seeds, timeout).await?;
    eprintln!(
        "migration-runner completed in {}ms",
        started.elapsed().as_millis()
    );
    Ok(summary)
}
