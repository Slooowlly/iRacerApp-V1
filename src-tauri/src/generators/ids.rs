use rusqlite::Connection;

use crate::db::connection::DbError;

// ── Tipos de entidade com ID sequencial ───────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum IdType {
    Driver,
    Team,
    Season,
    Race,
    Contract,
    News,
    Rivalry,
}

impl IdType {
    /// Chave na tabela meta que guarda o próximo contador.
    fn counter_key(self) -> &'static str {
        match self {
            IdType::Driver => "next_driver_id",
            IdType::Team => "next_team_id",
            IdType::Season => "next_season_id",
            IdType::Race => "next_race_id",
            IdType::Contract => "next_contract_id",
            IdType::News => "next_news_id",
            IdType::Rivalry => "next_rivalry_id",
        }
    }

    /// Prefixo legível do ID (ex: "P", "T", "RV").
    fn prefix(self) -> &'static str {
        match self {
            IdType::Driver => "P",
            IdType::Team => "T",
            IdType::Season => "S",
            IdType::Race => "R",
            IdType::Contract => "C",
            IdType::News => "N",
            IdType::Rivalry => "RV",
        }
    }
}

// ── Geração de IDs ────────────────────────────────────────────────────────────

/// Retorna o próximo ID sequencial para o tipo dado e incrementa o contador.
///
/// Exemplo: `next_id(conn, IdType::Driver)` → `"P001"`, depois `"P002"`, etc.
pub fn next_id(conn: &Connection, id_type: IdType) -> Result<String, DbError> {
    let key = id_type.counter_key();
    let prefix = id_type.prefix();

    let current: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .map_err(|_| DbError::NotFound(format!("Contador '{}' não encontrado em meta", key)))?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = ?2",
        rusqlite::params![(current + 1).to_string(), key],
    )?;

    Ok(format!("{}{:03}", prefix, current))
}

/// Gera `count` IDs sequenciais de uma vez (operação atômica).
///
/// Exemplo: `next_ids(conn, IdType::Driver, 3)` → `["P001", "P002", "P003"]`
pub fn next_ids(conn: &Connection, id_type: IdType, count: u32) -> Result<Vec<String>, DbError> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let key = id_type.counter_key();
    let prefix = id_type.prefix();

    let current: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .map_err(|_| DbError::NotFound(format!("Contador '{}' não encontrado em meta", key)))?;

    let next = current + count as i64;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = ?2",
        rusqlite::params![next.to_string(), key],
    )?;

    let ids = (current..next)
        .map(|n| format!("{}{:03}", prefix, n))
        .collect();

    Ok(ids)
}
