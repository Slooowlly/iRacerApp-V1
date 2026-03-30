use rusqlite::Connection;

use crate::db::connection::DbError;

/// Entrada de standings para uma categoria/temporada.
#[derive(Debug, Clone)]
pub struct StandingEntry {
    pub pilot_id: String,
    pub pilot_name: String,
    pub points: f64,
    pub position: i32,
}

/// Retorna a última vitória na carreira do piloto (qualquer categoria/temporada).
/// Retorna (season_num, round) ou None se nunca venceu.
pub fn get_last_career_win(
    conn: &Connection,
    pilot_id: &str,
) -> Result<Option<(i32, i32)>, DbError> {
    let result: Result<(i32, i32), _> = conn.query_row(
        "SELECT s.numero, c.rodada
         FROM race_results r
         JOIN calendar c ON r.race_id = c.id
         JOIN seasons s ON c.temporada_id = s.id
         WHERE r.piloto_id = ?1 AND r.posicao_final = 1
         ORDER BY s.numero DESC, c.rodada DESC
         LIMIT 1",
        rusqlite::params![pilot_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok(pair) => Ok(Some(pair)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DbError::Sqlite(e)),
    }
}

/// Retorna o número de vitórias do piloto com uma equipe específica (histórico completo).
pub fn get_wins_with_team(
    conn: &Connection,
    pilot_id: &str,
    team_id: &str,
) -> Result<i32, DbError> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*)
         FROM race_results
         WHERE piloto_id = ?1 AND equipe_id = ?2 AND posicao_final = 1",
        rusqlite::params![pilot_id, team_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Retorna os standings completos de uma categoria em uma temporada.
/// Ordenado por pontos (desc), posição calculada sequencialmente.
pub fn get_category_standings(
    conn: &Connection,
    temporada_id: &str,
    categoria: &str,
) -> Result<Vec<StandingEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT r.piloto_id, d.nome, SUM(r.pontos) as total_points
         FROM race_results r
         JOIN calendar c ON r.race_id = c.id
         JOIN drivers d ON r.piloto_id = d.id
         WHERE c.temporada_id = ?1 AND c.categoria = ?2
         GROUP BY r.piloto_id
         ORDER BY total_points DESC",
    )?;

    let mut standings = Vec::new();
    let mut rows = stmt.query(rusqlite::params![temporada_id, categoria])?;
    let mut position = 1;

    while let Some(row) = rows.next()? {
        standings.push(StandingEntry {
            pilot_id: row.get(0)?,
            pilot_name: row.get(1)?,
            points: row.get(2)?,
            position,
        });
        position += 1;
    }

    Ok(standings)
}

/// Retorna o número de vitórias do piloto na categoria esta temporada.
pub fn get_category_wins_this_season(
    conn: &Connection,
    pilot_id: &str,
    temporada_id: &str,
    categoria: &str,
) -> Result<i32, DbError> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*)
         FROM race_results r
         JOIN calendar c ON r.race_id = c.id
         WHERE r.piloto_id = ?1
           AND c.temporada_id = ?2
           AND c.categoria = ?3
           AND r.posicao_final = 1",
        rusqlite::params![pilot_id, temporada_id, categoria],
        |row| row.get(0),
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE seasons (id TEXT PRIMARY KEY, numero INTEGER NOT NULL);
            CREATE TABLE calendar (
                id TEXT PRIMARY KEY,
                temporada_id TEXT NOT NULL,
                rodada INTEGER NOT NULL,
                categoria TEXT NOT NULL
            );
            CREATE TABLE drivers (id TEXT PRIMARY KEY, nome TEXT NOT NULL);
            CREATE TABLE race_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                race_id TEXT NOT NULL,
                piloto_id TEXT NOT NULL,
                equipe_id TEXT NOT NULL,
                posicao_final INTEGER NOT NULL,
                pontos REAL NOT NULL
            );
            INSERT INTO seasons (id, numero) VALUES ('S1', 1), ('S2', 2);
            INSERT INTO drivers (id, nome) VALUES ('P001', 'Piloto Um'), ('P002', 'Piloto Dois');
            INSERT INTO calendar (id, temporada_id, rodada, categoria) VALUES
                ('R1', 'S1', 1, 'gt4'),
                ('R2', 'S1', 2, 'gt4'),
                ('R3', 'S2', 1, 'gt4');
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_get_last_career_win() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO race_results (race_id, piloto_id, equipe_id, posicao_final, pontos)
             VALUES ('R2', 'P001', 'T001', 1, 25.0)",
            [],
        )
        .unwrap();

        let result = get_last_career_win(&conn, "P001").unwrap();
        assert_eq!(result, Some((1, 2)));
    }

    #[test]
    fn test_get_last_career_win_none() {
        let conn = setup_db();
        let result = get_last_career_win(&conn, "P001").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_wins_with_team() {
        let conn = setup_db();
        conn.execute_batch(
            "INSERT INTO race_results (race_id, piloto_id, equipe_id, posicao_final, pontos) VALUES
             ('R1', 'P001', 'T001', 1, 25.0),
             ('R2', 'P001', 'T001', 1, 25.0),
             ('R3', 'P001', 'T002', 1, 25.0);",
        )
        .unwrap();

        assert_eq!(get_wins_with_team(&conn, "P001", "T001").unwrap(), 2);
        assert_eq!(get_wins_with_team(&conn, "P001", "T002").unwrap(), 1);
        assert_eq!(get_wins_with_team(&conn, "P001", "T003").unwrap(), 0);
    }

    #[test]
    fn test_get_category_standings() {
        let conn = setup_db();
        conn.execute_batch(
            "INSERT INTO race_results (race_id, piloto_id, equipe_id, posicao_final, pontos) VALUES
             ('R1', 'P001', 'T001', 1, 25.0),
             ('R1', 'P002', 'T002', 2, 18.0),
             ('R2', 'P001', 'T001', 2, 18.0),
             ('R2', 'P002', 'T002', 1, 25.0);",
        )
        .unwrap();

        let standings = get_category_standings(&conn, "S1", "gt4").unwrap();
        assert_eq!(standings.len(), 2);
        assert_eq!(standings[0].points, 43.0);
        assert_eq!(standings[0].position, 1);
        assert_eq!(standings[1].position, 2);
    }

    #[test]
    fn test_get_category_wins_this_season() {
        let conn = setup_db();
        conn.execute_batch(
            "INSERT INTO race_results (race_id, piloto_id, equipe_id, posicao_final, pontos) VALUES
             ('R1', 'P001', 'T001', 1, 25.0),
             ('R2', 'P001', 'T001', 2, 18.0);",
        )
        .unwrap();

        assert_eq!(
            get_category_wins_this_season(&conn, "P001", "S1", "gt4").unwrap(),
            1
        );
        assert_eq!(
            get_category_wins_this_season(&conn, "P001", "S2", "gt4").unwrap(),
            0
        );
    }
}
