use rusqlite::{params, Connection, OptionalExtension};

use crate::calendar::CalendarEntry;
use crate::db::connection::DbError;
use crate::models::enums::{RaceStatus, WeatherCondition};

pub fn insert_calendar_entry(conn: &Connection, entry: &CalendarEntry) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO calendar (
            id, temporada_id, season_id, categoria, rodada, nome,
            pista, track_id, track_name, track_config, clima, temperatura,
            voltas, duracao, duracao_corrida_min, duracao_classificacao_min,
            status, horario, data
        ) VALUES (
            :id, :temporada_id, :season_id, :categoria, :rodada, :nome,
            :pista, :track_id, :track_name, :track_config, :clima, :temperatura,
            :voltas, :duracao, :duracao_corrida_min, :duracao_classificacao_min,
            :status, :horario, :data
        )",
        rusqlite::named_params! {
            ":id": &entry.id,
            ":temporada_id": &entry.season_id,
            ":season_id": &entry.season_id,
            ":categoria": &entry.categoria,
            ":rodada": entry.rodada,
            ":nome": &entry.nome,
            ":pista": &entry.track_name,
            ":track_id": entry.track_id as i64,
            ":track_name": &entry.track_name,
            ":track_config": &entry.track_config,
            ":clima": entry.clima.as_str(),
            ":temperatura": entry.temperatura,
            ":voltas": entry.voltas,
            ":duracao": entry.duracao_corrida_min,
            ":duracao_corrida_min": entry.duracao_corrida_min,
            ":duracao_classificacao_min": entry.duracao_classificacao_min,
            ":status": entry.status.as_str(),
            ":horario": &entry.horario,
            ":data": "",
        },
    )?;
    Ok(())
}

pub fn insert_calendar_entries(
    conn: &Connection,
    entries: &[CalendarEntry],
) -> Result<(), DbError> {
    for entry in entries {
        insert_calendar_entry(conn, entry)?;
    }
    Ok(())
}

pub fn get_calendar(
    conn: &Connection,
    season_id: &str,
    categoria: &str,
) -> Result<Vec<CalendarEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1 AND categoria = ?2
         ORDER BY rodada ASC",
    )?;
    let mapped = stmt.query_map(params![season_id, categoria], calendar_from_row)?;
    collect_entries(mapped)
}

pub fn get_next_race(
    conn: &Connection,
    season_id: &str,
    categoria: &str,
) -> Result<Option<CalendarEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1
           AND categoria = ?2
           AND status = 'Pendente'
         ORDER BY rodada ASC
         LIMIT 1",
    )?;
    let entry = stmt
        .query_row(params![season_id, categoria], calendar_from_row)
        .optional()?;
    Ok(entry)
}

pub fn get_calendar_entry_by_id(
    conn: &Connection,
    id: &str,
) -> Result<Option<CalendarEntry>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM calendar WHERE id = ?1")?;
    let entry = stmt.query_row(params![id], calendar_from_row).optional()?;
    Ok(entry)
}

pub fn mark_race_completed(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE calendar SET status = 'Concluida' WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn get_pending_races(
    conn: &Connection,
    season_id: &str,
) -> Result<Vec<CalendarEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1
           AND status = 'Pendente'
         ORDER BY categoria ASC, rodada ASC",
    )?;
    let mapped = stmt.query_map(params![season_id], calendar_from_row)?;
    collect_entries(mapped)
}

pub fn get_pending_races_for_category(
    conn: &Connection,
    season_id: &str,
    category_id: &str,
) -> Result<Vec<CalendarEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1
           AND categoria = ?2
           AND status = 'Pendente'
         ORDER BY rodada ASC",
    )?;
    let mapped = stmt.query_map(params![season_id, category_id], calendar_from_row)?;
    collect_entries(mapped)
}

pub fn count_races_by_status(
    conn: &Connection,
    season_id: &str,
    categoria: &str,
    status: &RaceStatus,
) -> Result<i32, DbError> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1
           AND categoria = ?2
           AND status = ?3",
        params![season_id, categoria, status.as_str()],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn delete_calendar_for_season(conn: &Connection, season_id: &str) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM calendar WHERE COALESCE(season_id, temporada_id) = ?1",
        params![season_id],
    )?;
    Ok(())
}

fn collect_entries<T>(mapped: T) -> Result<Vec<CalendarEntry>, DbError>
where
    T: IntoIterator<Item = rusqlite::Result<CalendarEntry>>,
{
    let mut entries = Vec::new();
    for row in mapped {
        entries.push(row?);
    }
    Ok(entries)
}

fn calendar_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarEntry> {
    Ok(CalendarEntry {
        id: row.get("id")?,
        season_id: optional_string(row, "season_id")?
            .or_else(|| optional_string(row, "temporada_id").ok().flatten())
            .unwrap_or_default(),
        categoria: row.get("categoria")?,
        rodada: row.get("rodada")?,
        nome: optional_string(row, "nome")?.unwrap_or_else(|| {
            let pista = optional_string(row, "track_name")
                .ok()
                .flatten()
                .or_else(|| optional_string(row, "pista").ok().flatten())
                .unwrap_or_default();
            format!(
                "Rodada {} - {}",
                row.get::<_, i32>("rodada").unwrap_or(0),
                pista
            )
        }),
        track_id: optional_i64(row, "track_id")?.unwrap_or_default() as u32,
        track_name: optional_string(row, "track_name")?
            .or_else(|| optional_string(row, "pista").ok().flatten())
            .unwrap_or_default(),
        track_config: optional_string(row, "track_config")?.unwrap_or_default(),
        clima: WeatherCondition::from_str(&row.get::<_, String>("clima")?),
        temperatura: optional_f64(row, "temperatura")?.unwrap_or(25.0),
        voltas: optional_i64(row, "voltas")?.unwrap_or(10) as i32,
        duracao_corrida_min: optional_i64(row, "duracao_corrida_min")?
            .or_else(|| optional_i64(row, "duracao").ok().flatten())
            .unwrap_or(60) as i32,
        duracao_classificacao_min: optional_i64(row, "duracao_classificacao_min")?.unwrap_or(15)
            as i32,
        status: RaceStatus::from_str(
            &optional_string(row, "status")?.unwrap_or_else(|| "Pendente".to_string()),
        ),
        horario: optional_string(row, "horario")?.unwrap_or_else(|| "14:00".to_string()),
    })
}

fn optional_string(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<String>> {
    match row.get_ref(column_name)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        _ => row.get(column_name).map(Some),
    }
}

fn optional_i64(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<i64>> {
    match row.get_ref(column_name)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        _ => row.get(column_name).map(Some),
    }
}

fn optional_f64(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<f64>> {
    match row.get_ref(column_name)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        _ => row.get(column_name).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::calendar::generate_calendar_for_category;
    use crate::db::migrations;
    use crate::db::queries::seasons::insert_season;
    use crate::models::season::Season;

    #[test]
    fn test_insert_and_get_calendar() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");
        insert_season(&conn, &Season::new("S001".to_string(), 1, 2024)).expect("season");
        let mut rng = StdRng::seed_from_u64(10);
        let entries = generate_calendar_for_category("S001", "gt4", &mut rng).expect("calendar");

        insert_calendar_entries(&conn, &entries).expect("insert");
        let loaded = get_calendar(&conn, "S001", "gt4").expect("select");
        assert_eq!(loaded.len(), entries.len());
    }

    #[test]
    fn test_get_next_race() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");
        insert_season(&conn, &Season::new("S001".to_string(), 1, 2024)).expect("season");
        let mut rng = StdRng::seed_from_u64(11);
        let entries =
            generate_calendar_for_category("S001", "mazda_rookie", &mut rng).expect("calendar");

        insert_calendar_entries(&conn, &entries).expect("insert");
        let next = get_next_race(&conn, "S001", "mazda_rookie")
            .expect("next race")
            .expect("entry");
        assert_eq!(next.rodada, 1);
    }

    #[test]
    fn test_mark_race_completed() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");
        insert_season(&conn, &Season::new("S001".to_string(), 1, 2024)).expect("season");
        let mut rng = StdRng::seed_from_u64(12);
        let entry = generate_calendar_for_category("S001", "gt3", &mut rng)
            .expect("calendar")
            .into_iter()
            .next()
            .expect("entry");

        insert_calendar_entry(&conn, &entry).expect("insert");
        mark_race_completed(&conn, &entry.id).expect("update");

        let loaded = get_calendar_entry_by_id(&conn, &entry.id)
            .expect("select")
            .expect("entry");
        assert_eq!(loaded.status, RaceStatus::Concluida);
    }

    #[test]
    fn test_get_pending_races_for_category_filters_and_orders() {
        let conn = Connection::open_in_memory().expect("db");
        migrations::run_all(&conn).expect("schema");
        insert_season(&conn, &Season::new("S001".to_string(), 1, 2024)).expect("season");
        let mut rng = StdRng::seed_from_u64(13);

        let mazda_entries =
            generate_calendar_for_category("S001", "mazda_rookie", &mut rng).expect("mazda");
        let mut gt3_entries = generate_calendar_for_category("S001", "gt3", &mut rng).expect("gt3");
        for entry in &mut gt3_entries {
            entry.id = format!("gt3_{}", entry.id);
        }

        insert_calendar_entries(&conn, &mazda_entries).expect("insert mazda");
        insert_calendar_entries(&conn, &gt3_entries).expect("insert gt3");
        mark_race_completed(&conn, &gt3_entries[0].id).expect("complete gt3 round 1");

        let pending =
            get_pending_races_for_category(&conn, "S001", "gt3").expect("pending gt3 races");

        assert_eq!(pending.len(), gt3_entries.len() - 1);
        assert!(pending.iter().all(|entry| entry.categoria == "gt3"));
        assert_eq!(pending[0].rodada, 2);
    }
}
