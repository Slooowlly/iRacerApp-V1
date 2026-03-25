use rand::Rng;
use rusqlite::Connection;

use crate::db::queries::meta as meta_queries;

use crate::calendar::generate_all_calendars_with_id_factory;
use crate::constants::categories::get_all_categories;
use crate::db::queries::calendar as calendar_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::teams as team_queries;
use crate::generators::ids::{next_id, next_ids, IdType};
use crate::models::season::Season;

pub(crate) fn create_and_persist_new_season(
    conn: &Connection,
    season: &Season,
) -> Result<Season, String> {
    let new_season_id = next_id(conn, IdType::Season)
        .map_err(|e| format!("Falha ao gerar ID da nova temporada: {e}"))?;
    let new_year = season.ano + 1;
    let new_season = Season::new(new_season_id, season.numero + 1, new_year);
    season_queries::insert_season(conn, &new_season)
        .map_err(|e| format!("Falha ao inserir nova temporada: {e}"))?;
    Ok(new_season)
}

pub(crate) fn reset_driver_season_stats(conn: &Connection) -> Result<(), String> {
    let mut drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao recarregar pilotos apos a nova temporada: {e}"))?;
    for driver in &mut drivers {
        driver.reset_season_stats();
        driver_queries::update_driver(conn, driver)
            .map_err(|e| format!("Falha ao resetar stats do piloto '{}': {e}", driver.nome))?;
    }
    Ok(())
}

pub(crate) fn reset_team_season_stats(
    conn: &Connection,
    new_season_numero: i32,
) -> Result<(), String> {
    let teams = team_queries::get_all_teams(conn)
        .map_err(|e| format!("Falha ao recarregar equipes: {e}"))?;
    for team in &teams {
        team_queries::reset_team_season_stats(conn, &team.id)
            .map_err(|e| format!("Falha ao resetar stats da equipe '{}': {e}", team.id))?;
        conn.execute(
            "UPDATE teams SET temporada_atual = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            rusqlite::params![new_season_numero, &team.id],
        )
        .map_err(|e| format!("Falha ao atualizar temporada da equipe '{}': {e}", team.id))?;
    }
    Ok(())
}

pub(crate) fn seed_new_calendar(
    conn: &Connection,
    new_season_id: &str,
    new_year: i32,
    rng: &mut impl Rng,
) -> Result<(), String> {
    let total_new_races: u32 = get_all_categories()
        .iter()
        .map(|category| category.corridas_por_temporada as u32)
        .sum();
    let race_ids = next_ids(conn, IdType::Race, total_new_races)
        .map_err(|e| format!("Falha ao gerar IDs do calendario: {e}"))?;
    let mut race_ids_iter = race_ids.into_iter();
    let calendars = generate_all_calendars_with_id_factory(
        new_season_id,
        new_year,
        &mut || race_ids_iter.next().expect("calendar race id"),
        rng,
    )?;
    let all_entries: Vec<_> = calendars
        .values()
        .flat_map(|entries| entries.iter().cloned())
        .collect();
    calendar_queries::insert_calendar_entries(conn, &all_entries)
        .map_err(|e| format!("Falha ao inserir calendario da nova temporada: {e}"))?;
    Ok(())
}

pub(crate) fn update_meta_for_new_season(
    conn: &Connection,
    new_season_numero: i32,
    new_year: i32,
) -> Result<(), String> {
    meta_queries::set_current_season(conn, new_season_numero)
        .map_err(|e| format!("Falha ao atualizar meta current_season: {e}"))?;
    meta_queries::set_current_year(conn, new_year)
        .map_err(|e| format!("Falha ao atualizar meta current_year: {e}"))?;
    Ok(())
}
