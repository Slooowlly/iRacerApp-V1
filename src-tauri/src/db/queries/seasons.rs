use rusqlite::{params, Connection, OptionalExtension};

use crate::db::connection::DbError;
use crate::models::enums::{SeasonPhase, SeasonStatus};
use crate::models::season::Season;

pub fn insert_season(conn: &Connection, season: &Season) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO seasons (
            id, numero, ano, status, rodada_atual, fase, created_at, updated_at
        ) VALUES (
            :id, :numero, :ano, :status, :rodada_atual, :fase, :created_at, :updated_at
        )",
        rusqlite::named_params! {
            ":id": &season.id,
            ":numero": season.numero,
            ":ano": season.ano,
            ":status": season.status.as_str(),
            ":rodada_atual": season.rodada_atual,
            ":fase": season.fase.as_str(),
            ":created_at": &season.created_at,
            ":updated_at": &season.updated_at,
        },
    )?;
    Ok(())
}

pub fn get_season_by_id(conn: &Connection, id: &str) -> Result<Option<Season>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM seasons WHERE id = ?1")?;
    let season = stmt.query_row(params![id], season_from_row).optional()?;
    Ok(season)
}

pub fn get_active_season(conn: &Connection) -> Result<Option<Season>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM seasons
         WHERE status = 'EmAndamento'
         ORDER BY numero DESC
         LIMIT 1",
    )?;
    let season = stmt.query_row([], season_from_row).optional()?;
    Ok(season)
}

pub fn update_season_rodada(conn: &Connection, id: &str, rodada: i32) -> Result<(), DbError> {
    conn.execute(
        "UPDATE seasons
         SET rodada_atual = ?2, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![id, rodada],
    )?;
    Ok(())
}

pub fn finalize_season(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE seasons
         SET status = 'Finalizada', updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn get_all_seasons(conn: &Connection) -> Result<Vec<Season>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM seasons ORDER BY numero ASC")?;
    let mapped = stmt.query_map([], season_from_row)?;
    let mut seasons = Vec::new();
    for row in mapped {
        seasons.push(row?);
    }
    Ok(seasons)
}

fn season_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Season> {
    let fase_str = optional_string(row, "fase")?.unwrap_or_else(|| "BlocoRegular".to_string());
    let fase = SeasonPhase::from_str_strict(&fase_str)
        .map_err(rusqlite::Error::InvalidParameterName)?;

    Ok(Season {
        id: row.get("id")?,
        numero: row.get("numero")?,
        ano: row.get("ano")?,
        status: SeasonStatus::from_str_strict(&row.get::<_, String>("status")?)
            .map_err(rusqlite::Error::InvalidParameterName)?,
        rodada_atual: optional_i32(row, "rodada_atual")?.unwrap_or(1),
        fase,
        created_at: optional_string(row, "created_at")?.unwrap_or_default(),
        updated_at: optional_string(row, "updated_at")?.unwrap_or_default(),
    })
}

pub fn update_season_fase(
    conn: &Connection,
    id: &str,
    fase: &SeasonPhase,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE seasons
         SET fase = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![fase.as_str(), id],
    )?;
    Ok(())
}

fn optional_string(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<String>> {
    match row.get_ref(column_name)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        _ => row.get(column_name).map(Some),
    }
}

fn optional_i32(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<i32>> {
    match row.get_ref(column_name)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        _ => row.get(column_name).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::migrations;

    #[test]
    fn test_insert_and_get_season() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");

        let season = Season::new("S001".to_string(), 1, 2024);
        insert_season(&conn, &season).expect("insert");

        let loaded = get_season_by_id(&conn, "S001")
            .expect("select")
            .expect("season");
        assert_eq!(loaded.numero, 1);
        assert_eq!(loaded.status, SeasonStatus::EmAndamento);
        assert_eq!(loaded.fase, SeasonPhase::BlocoRegular);
    }

    #[test]
    fn test_get_active_and_finalize_season() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");

        let season = Season::new("S001".to_string(), 1, 2024);
        insert_season(&conn, &season).expect("insert");
        assert!(get_active_season(&conn).expect("active").is_some());

        finalize_season(&conn, "S001").expect("finalize");
        assert!(get_active_season(&conn).expect("active after").is_none());
    }
}
