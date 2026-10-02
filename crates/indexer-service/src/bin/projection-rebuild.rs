#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use std::{env, error::Error, io};
use storage::PgStore;

fn required_env(name: &str) -> Result<String, io::Error> {
    env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} is required")))
}

fn parse_u64(name: &str) -> Result<u64, Box<dyn Error + Send + Sync>> {
    Ok(required_env(name)?.parse::<u64>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be a u64: {error}"),
        )
    })?)
}

fn parse_batch_size() -> Result<usize, Box<dyn Error + Send + Sync>> {
    let value = required_env("PROJECTION_REBUILD_BATCH_SIZE")?;
    Ok(value.parse::<usize>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("PROJECTION_REBUILD_BATCH_SIZE must be a usize: {error}"),
        )
    })?)
}

fn hex_digest(digest: [u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let database_url = required_env("PROJECTION_REBUILD_DATABASE_URL")?;
    let stream_key = required_env("PROJECTION_REBUILD_STREAM_KEY")?;
    let start_slot = parse_u64("PROJECTION_REBUILD_START_SLOT")?;
    let end_slot = parse_u64("PROJECTION_REBUILD_END_SLOT")?;
    let batch_size = parse_batch_size()?;
    let store = PgStore::from_url(
        &database_url,
        2,
        storage::DEFAULT_POOL_WAIT_TIMEOUT,
        tax_engine::ENGINE_VERSION,
    )?;
    store.ping().await?;
    let report = store
        .rebuild_indexed_receipts_from_raw_evidence(&stream_key, start_slot, end_slot, batch_size)
        .await?;
    println!("scanned_rows={}", report.scanned_rows);
    println!("inserted_rows={}", report.inserted_rows);
    println!("duplicate_rows={}", report.duplicate_rows);
    println!("projection_digest={}", hex_digest(report.projection_digest));
    Ok(())
}
