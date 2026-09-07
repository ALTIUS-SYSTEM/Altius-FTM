//! Loads and applies the TypeQL schema, and provides typed query helpers.

use anyhow::Result;
use typedb_driver::{
    Addresses, Credentials, DriverOptions, DriverTlsConfig, TransactionType, TypeDBDriver,
};

pub const SCHEMA: &str = include_str!("../typeql/schema.tql");

/// Connect to a TypeDB 3.x server (plaintext — terminate TLS upstream).
pub async fn connect(address: &str, username: &str, password: &str) -> Result<TypeDBDriver> {
    let driver = TypeDBDriver::new(
        Addresses::try_from_address_str(address)?,
        Credentials::new(username, password),
        DriverOptions::new(DriverTlsConfig::disabled()),
    )
    .await?;
    Ok(driver)
}

/// Idempotently apply `SCHEMA` against `database` (creates it if missing).
/// Skips the exclusive schema transaction when the schema is already present,
/// so restarts don't stall behind open data transactions.
pub async fn migrate(driver: &TypeDBDriver, database: &str) -> Result<()> {
    if !driver.databases().contains(database).await? {
        driver.databases().create(database).await?;
    }

    if schema_present(driver, database).await {
        tracing::info!(database, "schema already present; skipping define");
        return Ok(());
    }

    let tx = driver.transaction(database, TransactionType::Schema).await?;
    tx.query(SCHEMA).await?;
    tx.commit().await?;
    tracing::info!(database, "schema applied");
    Ok(())
}

/// True when the marker entity type already exists in the database schema.
async fn schema_present(driver: &TypeDBDriver, database: &str) -> bool {
    match driver.transaction(database, TransactionType::Read).await {
        Ok(tx) => {
            let applied = tx
                .query("match entity $e sub organization; select $e;")
                .await
                .is_ok();
            tx.close().await.ok();
            applied
        }
        Err(_) => false,
    }
}
