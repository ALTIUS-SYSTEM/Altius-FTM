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

    // Always apply the definitions. `define` is declarative and idempotent:
    // re-stating an existing type is a no-op, while a type added since the
    // database was created gets defined now.
    //
    // Skipping when `organization` already existed meant every type added
    // after the first deployment — team, its relations, the user role cache —
    // silently never appeared, and the endpoints using them failed at runtime
    // on exactly the databases that had real data in them.
    let tx = driver
        .transaction(database, TransactionType::Schema)
        .await?;
    tx.query(SCHEMA).await?;
    tx.commit().await?;
    tracing::info!(database, "schema applied");
    Ok(())
}
