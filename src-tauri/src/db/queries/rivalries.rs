use rusqlite::{params, Connection, OptionalExtension};

use crate::db::connection::DbError;
use crate::models::rivalry::{perceived_intensity, Rivalry, RivalryType};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_rivalry(row: &rusqlite::Row) -> rusqlite::Result<Rivalry> {
    Ok(Rivalry {
        id: row.get(0)?,
        piloto1_id: row.get(1)?,
        piloto2_id: row.get(2)?,
        historical_intensity: row.get(3)?,
        recent_activity: row.get(4)?,
        tipo: RivalryType::from_str(&row.get::<_, String>(5)?),
        criado_em: row.get(6)?,
        ultima_atualizacao: row.get(7)?,
        temporada_update: row.get(8)?,
    })
}

const SELECT_COLS: &str = "id, piloto1_id, piloto2_id, historical_intensity, recent_activity, \
     tipo, criado_em, ultima_atualizacao, temporada_update";

// ── Leitura ───────────────────────────────────────────────────────────────────

/// Busca rivalidade pelo par normalizado (piloto1_id < piloto2_id).
pub fn get_rivalry_by_pair(
    conn: &Connection,
    piloto1_id: &str,
    piloto2_id: &str,
) -> Result<Option<Rivalry>, DbError> {
    let sql =
        format!("SELECT {SELECT_COLS} FROM rivalries WHERE piloto1_id = ?1 AND piloto2_id = ?2");
    conn.query_row(&sql, params![piloto1_id, piloto2_id], row_to_rivalry)
        .optional()
        .map_err(DbError::from)
}

/// Retorna todas as rivalidades de um piloto, ordenadas por intensidade percebida decrescente.
pub fn get_rivalries_for_pilot(conn: &Connection, pilot_id: &str) -> Result<Vec<Rivalry>, DbError> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM rivalries
         WHERE piloto1_id = ?1 OR piloto2_id = ?1
         ORDER BY (historical_intensity * 0.4 + recent_activity * 0.6) DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let iter = stmt.query_map(params![pilot_id], row_to_rivalry)?;
    let mut out = Vec::new();
    for r in iter {
        out.push(r?);
    }
    Ok(out)
}

/// Retorna todas as rivalidades existentes (usado no decaimento de fim de temporada).
pub fn get_all_rivalries(conn: &Connection) -> Result<Vec<Rivalry>, DbError> {
    let sql = format!("SELECT {SELECT_COLS} FROM rivalries");
    let mut stmt = conn.prepare(&sql)?;
    let iter = stmt.query_map([], row_to_rivalry)?;
    let mut out = Vec::new();
    for r in iter {
        out.push(r?);
    }
    Ok(out)
}

// ── Escrita ───────────────────────────────────────────────────────────────────

/// Insere uma rivalidade nova.
/// A coluna `intensidade` (legado) é mantida em sincronia com a percebida.
pub fn insert_rivalry(conn: &Connection, rivalry: &Rivalry) -> Result<(), DbError> {
    let perceived = rivalry.perceived_intensity();
    conn.execute(
        "INSERT INTO rivalries
             (id, piloto1_id, piloto2_id, intensidade, historical_intensity,
              recent_activity, tipo, criado_em, ultima_atualizacao, temporada_update)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            rivalry.id,
            rivalry.piloto1_id,
            rivalry.piloto2_id,
            perceived,
            rivalry.historical_intensity,
            rivalry.recent_activity,
            rivalry.tipo.as_str(),
            rivalry.criado_em,
            rivalry.ultima_atualizacao,
            rivalry.temporada_update,
        ],
    )?;
    Ok(())
}

/// Atualiza os dois eixos de intensidade e o timestamp de uma rivalidade existente.
/// A coluna `intensidade` (legado) é mantida em sincronia (= percebida).
/// O tipo original é preservado (identidade narrativa).
pub fn update_rivalry_axes(
    conn: &Connection,
    id: &str,
    historical_intensity: f64,
    recent_activity: f64,
    ultima_atualizacao: &str,
    temporada_update: i32,
) -> Result<(), DbError> {
    let perceived = perceived_intensity(historical_intensity, recent_activity);
    conn.execute(
        "UPDATE rivalries
         SET intensidade = ?1, historical_intensity = ?2, recent_activity = ?3,
             ultima_atualizacao = ?4, temporada_update = ?5
         WHERE id = ?6",
        params![
            perceived,
            historical_intensity,
            recent_activity,
            ultima_atualizacao,
            temporada_update,
            id,
        ],
    )?;
    Ok(())
}

/// Remove uma rivalidade pelo id.
pub fn delete_rivalry(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM rivalries WHERE id = ?1", params![id])?;
    Ok(())
}
