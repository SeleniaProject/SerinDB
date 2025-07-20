//! Rust SDK for SerinDB. Thin wrapper around tokio-postgres.

use tokio_postgres::{Client as PgClient, NoTls, Error, Row};

/// SerinDB async client.
pub struct Client {
    inner: PgClient,
}

impl Client {
    /// Connect to SerinDB using PostgreSQL wire address (e.g., "host=localhost user=alice").
    pub async fn connect(conn_str: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;
        // Spawn connection task.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        Ok(Self { inner: client })
    }

    /// Execute a query returning all rows.
    pub async fn query(&self, sql: &str) -> Result<Vec<Row>, Error> {
        self.inner.query(sql, &[]).await
    }

    /// Execute a statement without returning rows.
    pub async fn execute(&self, sql: &str) -> Result<u64, Error> {
        self.inner.execute(sql, &[]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_localhost() {
        // This test assumes local server running; skip if not reachable.
        if let Ok(cli) = Client::connect("host=127.0.0.1 user=alice password=password").await {
            let rows = cli.query("SELECT 1").await.unwrap();
            assert_eq!(rows[0].get::<usize, i32>(0), 1);
        }
    }
} 