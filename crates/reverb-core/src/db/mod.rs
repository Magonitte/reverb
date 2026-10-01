//! SQLite: abertura, PRAGMAs, migrações e acesso assíncrono (arquitetura §5).

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};

/// Migrações embutidas, em ordem. A versão fica em `PRAGMA user_version`.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/0001_init.sql"))];

/// Uma conexão de escrita protegida por `Mutex`; as chamadas rodam em `spawn_blocking`.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Abre (criando a pasta e o arquivo) e migra o banco.
    pub fn open(path: &Path) -> CoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    /// Banco em memória (testes que não precisam reabrir o arquivo).
    pub fn open_in_memory() -> CoreResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut conn: Connection) -> CoreResult<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Executa `f` numa thread de bloqueio com a conexão. Nunca segure o `Mutex` através de `.await`.
    pub async fn call<T, F>(&self, f: F) -> CoreResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> CoreResult<T> + Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let mut guard = conn
                .lock()
                .map_err(|_| CoreError::Internal("mutex do banco envenenado".into()))?;
            f(&mut guard)
        })
        .await
        .map_err(|e| CoreError::Internal(format!("tarefa do banco falhou: {e}")))?
    }

    pub async fn kv_get(&self, key: &str) -> CoreResult<Option<String>> {
        let key = key.to_string();
        self.call(move |conn| kv_get(conn, &key)).await
    }

    pub async fn kv_set(&self, key: &str, value: &str) -> CoreResult<()> {
        let (key, value) = (key.to_string(), value.to_string());
        self.call(move |conn| kv_set(conn, &key, &value)).await
    }

    /// Versão síncrona, para código que já roda em thread de bloqueio (CLI, testes).
    pub fn call_blocking<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| CoreError::Internal("mutex do banco envenenado".into()))?;
        f(&mut guard)
    }
}

/// Lê uma chave da tabela `kv` (estado interno: últimas verificações, autocura…).
pub fn kv_get(conn: &Connection, key: &str) -> CoreResult<Option<String>> {
    let mut stmt = conn.prepare_cached("SELECT value FROM kv WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn kv_set(conn: &Connection, key: &str, value: &str) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

/// Aplica, em ordem e dentro de transação, as migrações com versão > `user_version`.
pub fn migrate(conn: &mut Connection) -> CoreResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        tx.commit()?;
        tracing::info!(version, "migração aplicada");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
