use rusqlite::Transaction;
use crate::db::connection::DbError;
use crate::simulation::race::RaceDriverResult;

pub fn insert_race_results_batch(
    tx: &Transaction<'_>,
    race_id: &str,
    results: &[RaceDriverResult],
) -> Result<(), DbError> {
    let mut stmt = tx.prepare(
        "
        INSERT INTO race_results (
            race_id,
            piloto_id,
            equipe_id,
            posicao_largada,
            posicao_final,
            voltas_completadas,
            dnf,
            pontos,
            tempo_total,
            fastest_lap,
            dnf_reason,
            dnf_segment,
            incidents_count
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
        )
        ",
    )?;

    for result in results {
        let dnf_int = if result.is_dnf { 1 } else { 0 };
        let fastest_lap_int = if result.has_fastest_lap { 1 } else { 0 };
        
        stmt.execute(rusqlite::params![
            race_id,
            result.pilot_id,
            result.team_id,
            result.grid_position,
            result.finish_position,
            result.laps_completed,
            dnf_int,
            result.points_earned as f64,
            result.total_race_time_ms,
            fastest_lap_int,
            result.dnf_reason,
            result.dnf_segment,
            result.incidents_count
        ])?;
    }

    Ok(())
}
