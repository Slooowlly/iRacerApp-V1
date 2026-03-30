use rusqlite::Connection;

use crate::db::connection::DbError;
use crate::db::queries::drivers::get_driver;
use crate::db::queries::race_history::{
    get_category_standings, get_category_wins_this_season, get_last_career_win, get_wins_with_team,
    StandingEntry,
};
use crate::db::queries::rivalries::get_rivalries_for_pilot;
use crate::db::queries::teams::get_team_by_id;
use crate::db::queries::track_history::{get_pilot_dnf_at_track, TrackDnfRecord};
use crate::models::enums::ThematicSlot;
use crate::models::season::Season;
use crate::simulation::incidents::IncidentType;
use crate::simulation::race::RaceResult;

// ═══════════════════════════════════════════════════════════════════════════════
// Enums
// ═══════════════════════════════════════════════════════════════════════════════

/// Tier de performance da equipe, derivado de `car_performance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamPerformanceTier {
    Elite, // >= 90
    Alta,  // >= 80
    Media, // >= 70
    Baixa, // < 70
}

/// Clima narrativo da corrida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherNarrative {
    Dry,
    Wet,
    Changing,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Structs
// ═══════════════════════════════════════════════════════════════════════════════

/// Contexto narrativo completo do vencedor da corrida.
#[derive(Debug, Clone)]
pub struct WinnerNarrativeContext {
    // ── Identidade ──
    pub pilot_id: String,
    pub pilot_name: String,
    pub team_id: String,
    pub team_name: String,
    pub nationality: String,

    // ── Estilo da vitória (derivado in-memory do RaceResult) ──
    pub had_pole: bool,
    pub had_fastest_lap: bool,
    /// pole + win + fastest lap na mesma corrida
    pub is_grand_slam: bool,
    /// largou em P1 e venceu
    pub led_from_start: bool,
    pub gap_to_second_ms: f64,
    /// gap > 10 000 ms
    pub is_dominant_win: bool,
    /// 0 < gap < 500 ms
    pub is_photo_finish: bool,
    pub positions_gained: i32,
    /// largou em P5 ou pior e venceu
    pub is_comeback_win: bool,

    // ── Adversidade (derivado in-memory) ──
    pub had_incidents: bool,
    pub survived_collision: bool,
    pub collision_with_names: Vec<String>,
    /// desgaste de pneu > 85 % no fim da corrida
    pub high_tire_wear: bool,

    // ── Histórico (DB queries) ──
    pub career_wins_before: i32,
    pub season_wins_before: i32,
    pub is_first_career_win: bool,
    pub is_first_category_win: bool,
    pub is_first_win_with_team: bool,
    /// corridas desde a última vitória (None = nunca venceu antes)
    pub rounds_since_last_win: Option<i32>,
    /// jejum de 10+ corridas encerrado
    pub is_drought_end: bool,
    /// DNF anterior nesta pista (para redenção)
    pub previous_dnf_here: Option<TrackDnfRecord>,
    pub is_redemption: bool,

    // ── Contexto derivado inline ──
    pub is_home_race: bool,
    /// temporadas_na_categoria == 0
    pub is_category_rookie: bool,
    /// stats_carreira.temporadas == 0
    pub is_career_rookie: bool,
    pub team_performance_tier: TeamPerformanceTier,
    /// carro de tier Baixa ou Media
    pub is_underdog_win: bool,
    pub motivation: f64,

    // ── Rivalidade ──
    pub beat_rival: bool,
    pub rival_beaten_name: Option<String>,

    // ── Milestones ──
    /// 10 / 25 / 50 / 75 / 100 / 150 / 200 vitórias
    pub milestone_wins: Option<i32>,
}

/// Contexto narrativo completo da corrida, usado pelo gerador de notícias.
#[derive(Debug, Clone)]
pub struct RaceNarrativeContext {
    pub season_num: i32,
    pub round: i32,
    pub total_rounds: i32,
    pub category: String,
    pub track_name: String,
    pub thematic_slot: ThematicSlot,
    pub winner: WinnerNarrativeContext,
    pub weather: WeatherNarrative,
    /// Standings antes desta corrida (baseado em race_results anteriores)
    pub standings_before: Vec<StandingEntry>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Builder
// ═══════════════════════════════════════════════════════════════════════════════

/// Constrói o `RaceNarrativeContext` cruzando dados in-memory com queries ao banco.
/// Deve ser chamado após salvar os resultados da corrida no banco.
pub fn build_race_narrative_context(
    conn: &Connection,
    race_result: &RaceResult,
    active_season: &Season,
    round: i32,
    total_rounds: i32,
    category: &str,
    thematic_slot: ThematicSlot,
) -> Result<RaceNarrativeContext, DbError> {
    let season_num = active_season.numero;
    let temporada_id = &active_season.id;

    // ── Vencedor ──
    let winner_result = race_result
        .race_results
        .iter()
        .find(|r| r.finish_position == 1)
        .ok_or_else(|| DbError::NotFound("No winner found in race results".into()))?;

    let winner_driver = get_driver(conn, &winner_result.pilot_id)?;
    let winner_team = get_team_by_id(conn, &winner_result.team_id)?
        .ok_or_else(|| DbError::NotFound(format!("Team not found: {}", winner_result.team_id)))?;

    // ── Gap para o segundo colocado ──
    let gap_to_second = race_result
        .race_results
        .iter()
        .find(|r| r.finish_position == 2)
        .map(|r| r.gap_to_winner_ms)
        .unwrap_or(f64::MAX);

    // ── Colisões do vencedor ──
    let collision_with_ids: Vec<String> = winner_result
        .incidents
        .iter()
        .filter(|inc| inc.incident_type == IncidentType::Collision)
        .filter_map(|inc| inc.linked_pilot_id.clone())
        .collect();

    let collision_with_names: Vec<String> = collision_with_ids
        .iter()
        .filter_map(|id| {
            race_result
                .race_results
                .iter()
                .find(|r| &r.pilot_id == id)
                .map(|r| r.pilot_name.clone())
        })
        .collect();

    // ── Histórico: última vitória na carreira ──
    let last_win = get_last_career_win(conn, &winner_result.pilot_id)?;
    let rounds_since_last_win = last_win.map(|(win_season, win_round)| {
        if win_season == season_num {
            round - win_round
        } else {
            // Aproximação: 12 corridas por temporada
            (season_num - win_season) * 12 + round
        }
    });

    // ── Histórico: vitórias com a equipe atual ──
    let wins_with_team = get_wins_with_team(conn, &winner_result.pilot_id, &winner_result.team_id)?;

    // ── Histórico: vitórias na categoria nesta temporada ──
    let category_wins_this_season =
        get_category_wins_this_season(conn, &winner_result.pilot_id, temporada_id, category)?;

    // ── Histórico: DNF anterior nesta pista ──
    let previous_dnf =
        get_pilot_dnf_at_track(conn, &winner_result.pilot_id, &race_result.track_name)?;

    // ── Rivalidades ──
    let rivalries = get_rivalries_for_pilot(conn, &winner_result.pilot_id).unwrap_or_default();
    let active_rival_ids: Vec<String> = rivalries
        .iter()
        .filter(|r| r.perceived_intensity() >= 20.0)
        .map(|r| {
            if r.piloto1_id == winner_result.pilot_id {
                r.piloto2_id.clone()
            } else {
                r.piloto1_id.clone()
            }
        })
        .collect();

    let beat_rival_info = active_rival_ids.iter().find_map(|rival_id| {
        race_result
            .race_results
            .iter()
            .find(|r| &r.pilot_id == rival_id)
            .filter(|r| r.finish_position > 1 || r.is_dnf)
            .map(|r| r.pilot_name.clone())
    });

    // ── Contexto derivado inline ──
    let weather = parse_weather_narrative(&race_result.weather);
    let team_tier = derive_team_performance_tier(winner_team.car_performance);
    let is_home = is_home_race(&winner_driver.nacionalidade, &race_result.track_name);
    let standings_before = get_category_standings(conn, temporada_id, category)?;

    // ── Milestone check ──
    let career_wins_before = winner_driver.stats_carreira.vitorias as i32;
    let new_career_wins = career_wins_before + 1;
    let milestone = match new_career_wins {
        10 | 25 | 50 | 75 | 100 | 150 | 200 => Some(new_career_wins),
        _ => None,
    };

    let winner_ctx = WinnerNarrativeContext {
        pilot_id: winner_result.pilot_id.clone(),
        pilot_name: winner_result.pilot_name.clone(),
        team_id: winner_result.team_id.clone(),
        team_name: winner_result.team_name.clone(),
        nationality: winner_driver.nacionalidade.clone(),

        had_pole: race_result.pole_sitter_id == winner_result.pilot_id,
        had_fastest_lap: race_result.fastest_lap_id == winner_result.pilot_id,
        is_grand_slam: race_result.pole_sitter_id == winner_result.pilot_id
            && race_result.fastest_lap_id == winner_result.pilot_id
            && winner_result.grid_position == 1,
        led_from_start: winner_result.grid_position == 1,
        gap_to_second_ms: gap_to_second,
        is_dominant_win: gap_to_second >= 10_000.0,
        is_photo_finish: gap_to_second > 0.0 && gap_to_second < 500.0,
        positions_gained: winner_result.positions_gained,
        is_comeback_win: winner_result.grid_position >= 5,

        had_incidents: winner_result.incidents_count > 0,
        survived_collision: !collision_with_ids.is_empty(),
        collision_with_names,
        high_tire_wear: winner_result.final_tire_wear > 0.85,

        career_wins_before,
        season_wins_before: winner_driver.stats_temporada.vitorias as i32,
        is_first_career_win: career_wins_before == 0,
        is_first_category_win: winner_driver.temporadas_na_categoria == 0
            && category_wins_this_season == 0,
        is_first_win_with_team: wins_with_team == 0,
        rounds_since_last_win,
        is_drought_end: rounds_since_last_win.map(|r| r >= 10).unwrap_or(false),

        previous_dnf_here: previous_dnf.clone(),
        is_redemption: previous_dnf.is_some(),

        is_home_race: is_home,
        is_category_rookie: winner_driver.temporadas_na_categoria == 0,
        is_career_rookie: winner_driver.stats_carreira.temporadas == 0,
        team_performance_tier: team_tier,
        is_underdog_win: matches!(
            team_tier,
            TeamPerformanceTier::Baixa | TeamPerformanceTier::Media
        ),
        motivation: winner_driver.motivacao,

        beat_rival: beat_rival_info.is_some(),
        rival_beaten_name: beat_rival_info,

        milestone_wins: milestone,
    };

    Ok(RaceNarrativeContext {
        season_num,
        round,
        total_rounds,
        category: category.to_string(),
        track_name: race_result.track_name.clone(),
        thematic_slot,
        winner: winner_ctx,
        weather,
        standings_before,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

pub fn derive_team_performance_tier(car_performance: f64) -> TeamPerformanceTier {
    if car_performance >= 90.0 {
        TeamPerformanceTier::Elite
    } else if car_performance >= 80.0 {
        TeamPerformanceTier::Alta
    } else if car_performance >= 70.0 {
        TeamPerformanceTier::Media
    } else {
        TeamPerformanceTier::Baixa
    }
}

pub fn parse_weather_narrative(weather: &str) -> WeatherNarrative {
    let lower = weather.to_lowercase();
    if lower.contains("wet") || lower.contains("rain") || lower.contains("chuva") {
        WeatherNarrative::Wet
    } else if lower.contains("chang") || lower.contains("variav") {
        WeatherNarrative::Changing
    } else {
        WeatherNarrative::Dry
    }
}

/// Verifica se a corrida é "em casa" para o piloto, comparando nacionalidade com o país da pista.
pub fn is_home_race(nationality: &str, track_name: &str) -> bool {
    track_country(track_name)
        .map(|countries| {
            countries
                .iter()
                .any(|c| nationality.to_lowercase().contains(&c.to_lowercase()))
        })
        .unwrap_or(false)
}

/// Retorna as formas aceitas de representar o país de uma pista (ISO, PT, EN).
fn track_country(track_name: &str) -> Option<&'static [&'static str]> {
    let name = track_name.to_lowercase();

    if name.contains("interlagos") || name.contains("sao paulo") || name.contains("são paulo") {
        return Some(&["BR", "Brasil", "Brazil"]);
    }
    if name.contains("spa") {
        return Some(&["BE", "Bélgica", "Belgium", "Belgica"]);
    }
    if name.contains("monza") || name.contains("imola") || name.contains("mugello") {
        return Some(&["IT", "Itália", "Italy", "Italia"]);
    }
    if name.contains("silverstone") || name.contains("brands") || name.contains("donington") {
        return Some(&[
            "GB",
            "UK",
            "Reino Unido",
            "United Kingdom",
            "Inglaterra",
            "England",
        ]);
    }
    if name.contains("nurburgring") || name.contains("nürburgring") || name.contains("hockenheim")
    {
        return Some(&["DE", "Alemanha", "Germany"]);
    }
    if name.contains("suzuka") || name.contains("fuji") || name.contains("motegi") {
        return Some(&["JP", "Japão", "Japan", "Japao"]);
    }
    if name.contains("laguna")
        || name.contains("road america")
        || name.contains("watkins")
        || name.contains("daytona")
        || name.contains("sebring")
        || name.contains("cota")
        || name.contains("indianapolis")
        || name.contains("long beach")
    {
        return Some(&["US", "USA", "Estados Unidos", "United States", "EUA"]);
    }
    if name.contains("barcelona") || name.contains("catalunya") || name.contains("jerez") {
        return Some(&["ES", "Espanha", "Spain"]);
    }
    if name.contains("le mans") || name.contains("paul ricard") || name.contains("magny") {
        return Some(&["FR", "França", "France", "Franca"]);
    }
    if name.contains("zandvoort") {
        return Some(&["NL", "Holanda", "Netherlands", "Países Baixos"]);
    }
    if name.contains("red bull ring") || name.contains("spielberg") {
        return Some(&["AT", "Áustria", "Austria"]);
    }
    if name.contains("montreal") || name.contains("mosport") {
        return Some(&["CA", "Canadá", "Canada"]);
    }
    if name.contains("melbourne") || name.contains("phillip island") || name.contains("bathurst") {
        return Some(&["AU", "Austrália", "Australia"]);
    }
    if name.contains("portimao") || name.contains("estoril") {
        return Some(&["PT", "Portugal"]);
    }
    if name.contains("mexico") || name.contains("méxico") {
        return Some(&["MX", "México", "Mexico"]);
    }
    if name.contains("buenos aires") || name.contains("argentina") {
        return Some(&["AR", "Argentina"]);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_performance_tier() {
        assert_eq!(
            derive_team_performance_tier(95.0),
            TeamPerformanceTier::Elite
        );
        assert_eq!(
            derive_team_performance_tier(90.0),
            TeamPerformanceTier::Elite
        );
        assert_eq!(
            derive_team_performance_tier(85.0),
            TeamPerformanceTier::Alta
        );
        assert_eq!(
            derive_team_performance_tier(80.0),
            TeamPerformanceTier::Alta
        );
        assert_eq!(
            derive_team_performance_tier(75.0),
            TeamPerformanceTier::Media
        );
        assert_eq!(
            derive_team_performance_tier(70.0),
            TeamPerformanceTier::Media
        );
        assert_eq!(
            derive_team_performance_tier(65.0),
            TeamPerformanceTier::Baixa
        );
        assert_eq!(
            derive_team_performance_tier(0.0),
            TeamPerformanceTier::Baixa
        );
    }

    #[test]
    fn test_weather_narrative() {
        assert_eq!(parse_weather_narrative("Dry"), WeatherNarrative::Dry);
        assert_eq!(parse_weather_narrative("Clear"), WeatherNarrative::Dry);
        assert_eq!(parse_weather_narrative("Wet"), WeatherNarrative::Wet);
        assert_eq!(parse_weather_narrative("Rain"), WeatherNarrative::Wet);
        assert_eq!(parse_weather_narrative("Chuva"), WeatherNarrative::Wet);
        assert_eq!(
            parse_weather_narrative("Changing"),
            WeatherNarrative::Changing
        );
        assert_eq!(
            parse_weather_narrative("Variavel"),
            WeatherNarrative::Changing
        );
    }

    #[test]
    fn test_is_home_race() {
        assert!(is_home_race("Brasil", "Interlagos"));
        assert!(is_home_race("BR", "Sao Paulo"));
        assert!(is_home_race("Brazil", "Interlagos"));
        assert!(is_home_race("Alemanha", "Nurburgring"));
        assert!(is_home_race("Germany", "Hockenheim"));
        assert!(is_home_race("Japão", "Suzuka"));

        assert!(!is_home_race("Brasil", "Spa"));
        assert!(!is_home_race("Alemanha", "Monza"));
        assert!(!is_home_race("France", "Unknown Track"));
    }

    #[test]
    fn test_track_country_known() {
        assert!(track_country("Interlagos").is_some());
        assert!(track_country("Spa-Francorchamps").is_some());
        assert!(track_country("Monza").is_some());
        assert!(track_country("Silverstone").is_some());
        assert!(track_country("Suzuka").is_some());
    }

    #[test]
    fn test_track_country_unknown() {
        assert!(track_country("Unknown Track").is_none());
        assert!(track_country("").is_none());
    }
}
