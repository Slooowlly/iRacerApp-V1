use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::constants::scoring::{get_weather_penalty, RACE_SCORE_TO_LAP_MS};
use crate::models::enums::WeatherCondition;

use super::context::{SimDriver, SimulationContext};
use super::incidents::{process_segment_incidents, IncidentResult};
use super::qualifying::QualifyingResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RaceSegment {
    Start,
    Early,
    Mid,
    Late,
    Finish,
}

impl RaceSegment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "START",
            Self::Early => "EARLY",
            Self::Mid => "MID",
            Self::Late => "LATE",
            Self::Finish => "FINISH",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaceState {
    pub driver_id: String,
    pub tire_wear: f64,
    pub physical_condition: f64,
    pub cumulative_score: f64,
    pub is_dnf: bool,
    pub current_position: i32,
    pub incidents: Vec<IncidentResult>,
    pub dnf_reason: Option<String>,
    pub dnf_segment: Option<RaceSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceDriverResult {
    pub pilot_id: String,
    pub pilot_name: String,
    pub team_id: String,
    pub team_name: String,
    pub grid_position: i32,
    pub finish_position: i32,
    pub positions_gained: i32,
    pub best_lap_time_ms: f64,
    pub total_race_time_ms: f64,
    pub gap_to_winner_ms: f64,
    pub is_dnf: bool,
    pub dnf_reason: Option<String>,
    pub dnf_segment: Option<String>,
    #[serde(default)]
    pub incidents_count: i32,
    #[serde(default)]
    pub incidents: Vec<IncidentResult>,
    pub has_fastest_lap: bool,
    pub points_earned: i32,
    pub is_jogador: bool,
    pub laps_completed: i32,
    pub final_tire_wear: f64,
    pub final_physical: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResult {
    pub qualifying_results: Vec<QualifyingResult>,
    pub race_results: Vec<RaceDriverResult>,
    pub pole_sitter_id: String,
    pub winner_id: String,
    pub fastest_lap_id: String,
    pub total_laps: i32,
    pub weather: String,
    pub track_name: String,
    #[serde(default)]
    pub total_incidents: i32,
    #[serde(default)]
    pub total_dnfs: i32,
}

pub fn simulate_race(
    drivers: &[SimDriver],
    qualifying: &[QualifyingResult],
    ctx: &SimulationContext,
    rng: &mut impl Rng,
) -> RaceResult {
    let total_drivers = qualifying.len() as i32;
    let mut states: Vec<RaceState> = qualifying
        .iter()
        .map(|result| RaceState {
            driver_id: result.pilot_id.clone(),
            tire_wear: 1.0,
            physical_condition: 1.0,
            cumulative_score: (total_drivers - result.position + 1) as f64 * 2.0,
            is_dnf: false,
            current_position: result.position,
            incidents: Vec::new(),
            dnf_reason: None,
            dnf_segment: None,
        })
        .collect();

    for segment in [
        RaceSegment::Start,
        RaceSegment::Early,
        RaceSegment::Mid,
        RaceSegment::Late,
        RaceSegment::Finish,
    ] {
        if ctx.incidents_enabled {
            let segment_incidents = process_segment_incidents(
                drivers,
                &states,
                segment,
                ctx.weather,
                ctx.is_championship_deciding,
                rng,
            );

            for incident in segment_incidents {
                if let Some(state) = states.iter_mut().find(|s| s.driver_id == incident.pilot_id) {
                    if incident.is_dnf {
                        state.is_dnf = true;
                        state.dnf_reason = Some(incident.description.clone());
                        state.dnf_segment = Some(segment);
                    }
                    state.incidents.push(incident);
                }
            }
        }

        let seg_str = segment.as_str();
        for state in &mut states {
            if state.is_dnf {
                continue;
            }

            if let Some(driver) = drivers.iter().find(|driver| driver.id == state.driver_id) {
                let mut segment_score = calculate_segment_score(driver, state, segment, ctx, rng);
                let penalty: f64 = state
                    .incidents
                    .iter()
                    .filter(|incident| incident.segment == seg_str && !incident.is_dnf)
                    .map(|incident| incident.positions_lost as f64 * 2.0)
                    .sum();
                segment_score = (segment_score - penalty).max(0.0);
                state.cumulative_score += segment_score;
                apply_tire_degradation(state, driver, ctx);
                apply_physical_degradation(state, driver, ctx);
            }
        }

        states.sort_by(|a, b| match (a.is_dnf, b.is_dnf) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => b
                .cumulative_score
                .partial_cmp(&a.cumulative_score)
                .unwrap_or(std::cmp::Ordering::Equal),
        });

        for (index, state) in states.iter_mut().enumerate() {
            state.current_position = index as i32 + 1;
        }
    }

    let mut race_results = build_race_results(drivers, qualifying, ctx, &states, rng);
    let pole_sitter_id = qualifying
        .first()
        .map(|result| result.pilot_id.clone())
        .unwrap_or_default();
    let winner_id = race_results
        .first()
        .map(|result| result.pilot_id.clone())
        .unwrap_or_default();
    let total_incidents = race_results
        .iter()
        .map(|result| result.incidents_count)
        .sum();
    let total_dnfs = race_results.iter().filter(|result| result.is_dnf).count() as i32;

    RaceResult {
        qualifying_results: qualifying.to_vec(),
        race_results: std::mem::take(&mut race_results),
        pole_sitter_id,
        winner_id,
        fastest_lap_id: String::new(),
        total_laps: ctx.total_laps,
        weather: ctx.weather.as_str().to_string(),
        track_name: ctx.track_name.clone(),
        total_incidents,
        total_dnfs,
    }
}

fn build_race_results(
    drivers: &[SimDriver],
    qualifying: &[QualifyingResult],
    ctx: &SimulationContext,
    states: &[RaceState],
    rng: &mut impl Rng,
) -> Vec<RaceDriverResult> {
    let winner_score = states
        .first()
        .map(|state| state.cumulative_score)
        .unwrap_or(0.0);
    let winner_lap_time_ms = ctx.base_lap_time_ms;
    let winner_total_time_ms = winner_lap_time_ms * ctx.total_laps as f64;

    let mut results: Vec<RaceDriverResult> = states
        .iter()
        .filter_map(|state| {
            let driver = drivers.iter().find(|driver| driver.id == state.driver_id)?;
            let qualifying_result = qualifying
                .iter()
                .find(|result| result.pilot_id == state.driver_id)?;
            let lap_time_ms =
                ctx.base_lap_time_ms + (winner_score - state.cumulative_score).max(0.0) * RACE_SCORE_TO_LAP_MS;
            let total_race_time_ms = lap_time_ms * ctx.total_laps as f64;
            let best_lap_factor = rng.gen_range(0.97..=1.0);
            let best_lap_time_ms = lap_time_ms * best_lap_factor;
            let laps_completed = if state.is_dnf {
                estimate_laps_at_dnf(state.dnf_segment, ctx.total_laps)
            } else {
                ctx.total_laps
            };

            Some(RaceDriverResult {
                pilot_id: driver.id.clone(),
                pilot_name: driver.nome.clone(),
                team_id: driver.team_id.clone(),
                team_name: driver.team_name.clone(),
                grid_position: qualifying_result.position,
                finish_position: state.current_position,
                positions_gained: qualifying_result.position - state.current_position,
                best_lap_time_ms,
                total_race_time_ms,
                gap_to_winner_ms: (total_race_time_ms - winner_total_time_ms).max(0.0),
                is_dnf: state.is_dnf,
                dnf_reason: state.dnf_reason.clone(),
                dnf_segment: state
                    .dnf_segment
                    .map(|segment| segment.as_str().to_string()),
                incidents_count: state.incidents.len() as i32,
                incidents: state.incidents.clone(),
                has_fastest_lap: false,
                points_earned: 0,
                is_jogador: driver.is_jogador,
                laps_completed,
                final_tire_wear: state.tire_wear,
                final_physical: state.physical_condition,
            })
        })
        .collect();

    results.sort_by_key(|result| result.finish_position);
    results
}

fn estimate_laps_at_dnf(segment: Option<RaceSegment>, total_laps: i32) -> i32 {
    let fraction = match segment {
        Some(RaceSegment::Start) => 0.10,
        Some(RaceSegment::Early) => 0.30,
        Some(RaceSegment::Mid) => 0.50,
        Some(RaceSegment::Late) => 0.70,
        Some(RaceSegment::Finish) => 0.90,
        None => 1.0,
    };
    ((total_laps as f64 * fraction) as i32).max(1)
}

#[derive(Debug, Clone, Copy)]
struct SegmentWeights {
    skill: f64,
    habilidade_largada: f64,
    racecraft: f64,
    car_performance: f64,
    gestao_pneus: f64,
    fitness: f64,
    mentalidade: f64,
    confianca: f64,
}

fn segment_weights(segment: RaceSegment) -> SegmentWeights {
    match segment {
        RaceSegment::Start => SegmentWeights {
            skill: 0.20,
            habilidade_largada: 0.35,
            racecraft: 0.25,
            car_performance: 0.20,
            gestao_pneus: 0.0,
            fitness: 0.0,
            mentalidade: 0.0,
            confianca: 0.0,
        },
        RaceSegment::Early => SegmentWeights {
            skill: 0.35,
            habilidade_largada: 0.0,
            racecraft: 0.20,
            car_performance: 0.30,
            gestao_pneus: 0.15,
            fitness: 0.0,
            mentalidade: 0.0,
            confianca: 0.0,
        },
        RaceSegment::Mid => SegmentWeights {
            skill: 0.35,
            habilidade_largada: 0.0,
            racecraft: 0.0,
            car_performance: 0.30,
            gestao_pneus: 0.20,
            fitness: 0.15,
            mentalidade: 0.0,
            confianca: 0.0,
        },
        RaceSegment::Late => SegmentWeights {
            skill: 0.25,
            habilidade_largada: 0.0,
            racecraft: 0.0,
            car_performance: 0.20,
            gestao_pneus: 0.25,
            fitness: 0.20,
            mentalidade: 0.10,
            confianca: 0.0,
        },
        RaceSegment::Finish => SegmentWeights {
            skill: 0.25,
            habilidade_largada: 0.0,
            racecraft: 0.25,
            car_performance: 0.20,
            gestao_pneus: 0.0,
            fitness: 0.0,
            mentalidade: 0.10,
            confianca: 0.20,
        },
    }
}

fn calculate_segment_score(
    driver: &SimDriver,
    state: &RaceState,
    segment: RaceSegment,
    ctx: &SimulationContext,
    rng: &mut impl Rng,
) -> f64 {
    let weights = segment_weights(segment);
    let mut score = driver.skill as f64 * weights.skill
        + driver.habilidade_largada as f64 * weights.habilidade_largada
        + driver.racecraft as f64 * weights.racecraft
        + normalize_car_performance(driver.car_performance) * weights.car_performance
        + driver.gestao_pneus as f64 * weights.gestao_pneus
        + driver.fitness as f64 * weights.fitness
        + driver.mentalidade as f64 * weights.mentalidade
        + driver.confianca as f64 * weights.confianca;

    let tire_penalty = (1.0 - state.tire_wear) * 0.15;
    score *= 1.0 - tire_penalty;

    if matches!(segment, RaceSegment::Late | RaceSegment::Finish) {
        let fatigue_penalty = (1.0 - state.physical_condition) * 0.10;
        score *= 1.0 - fatigue_penalty;
    }

    score *= weather_multiplier(ctx.weather, driver.fator_chuva);

    let variance_range = (100.0 - driver.consistencia as f64) / 100.0 * 5.0;
    score += rng.gen_range(-variance_range..=variance_range);

    score.max(5.0)
}

fn apply_tire_degradation(state: &mut RaceState, driver: &SimDriver, ctx: &SimulationContext) {
    let mgmt_factor = 1.0 - (driver.gestao_pneus as f64 / 100.0 * 0.50);
    let duration_factor = (ctx.race_duration_minutes as f64 / 30.0).max(0.25);
    let actual_degradation = ctx.tire_degradation_rate * mgmt_factor * duration_factor;
    state.tire_wear = (state.tire_wear - actual_degradation).max(0.1);
}

fn apply_physical_degradation(state: &mut RaceState, driver: &SimDriver, ctx: &SimulationContext) {
    let fit_factor = 1.0 - (driver.fitness as f64 / 100.0 * 0.60);
    let duration_factor = (ctx.race_duration_minutes as f64 / 30.0).max(0.25);
    let actual_degradation = ctx.physical_degradation_rate * fit_factor * duration_factor;
    state.physical_condition = (state.physical_condition - actual_degradation).max(0.2);
}

fn weather_multiplier(weather: WeatherCondition, fator_chuva: u8) -> f64 {
    if weather == WeatherCondition::Dry {
        return 1.0;
    }

    let (base_penalty, _) = get_weather_penalty(&weather);
    let absorption = fator_chuva as f64 / 100.0 * 0.90;
    let rain_penalty = base_penalty * (1.0 - absorption);
    1.0 - rain_penalty
}

fn normalize_car_performance(car_performance: f64) -> f64 {
    ((car_performance + 5.0) / 21.0 * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::models::driver::Driver;
    use crate::models::team::placeholder_team_from_db;

    use super::*;
    use crate::simulation::qualifying::simulate_qualifying;

    fn sample_context(duration: i32, weather: WeatherCondition) -> SimulationContext {
        SimulationContext {
            category_id: "gt4".to_string(),
            category_tier: 3,
            track_id: 2,
            track_name: "Interlagos".to_string(),
            weather,
            temperature: 24.0,
            total_laps: 18,
            race_duration_minutes: duration,
            is_championship_deciding: false,
            base_lap_time_ms: 90_000.0,
            tire_degradation_rate: 0.02,
            physical_degradation_rate: 0.01,
            incidents_enabled: false,
        }
    }

    fn sample_context_with_incidents(
        duration: i32,
        weather: WeatherCondition,
    ) -> SimulationContext {
        SimulationContext {
            incidents_enabled: true,
            ..sample_context(duration, weather)
        }
    }

    fn build_driver(
        id: &str,
        skill: f64,
        racecraft: f64,
        pneus: f64,
        fitness: f64,
        car: f64,
    ) -> SimDriver {
        let mut driver = Driver::create_player(
            id.to_string(),
            format!("Driver {}", id),
            "🇧🇷 Brasileiro".to_string(),
            20,
        );
        driver.is_jogador = false;
        driver.atributos.skill = skill;
        driver.atributos.consistencia = 88.0;
        driver.atributos.racecraft = racecraft;
        driver.atributos.ritmo_classificacao = skill;
        driver.atributos.habilidade_largada = 70.0;
        driver.atributos.gestao_pneus = pneus;
        driver.atributos.fitness = fitness;
        driver.atributos.mentalidade = 72.0;
        driver.atributos.confianca = 70.0;
        driver.atributos.adaptabilidade = 68.0;
        driver.atributos.fator_chuva = 50.0;

        let mut team = placeholder_team_from_db(
            format!("T{}", id),
            format!("Team {}", id),
            "gt4".to_string(),
            "2026-01-01T00:00:00".to_string(),
        );
        team.car_performance = car;
        team.confiabilidade = 80.0;

        SimDriver::from_driver_and_team(&driver, &team)
    }

    fn build_grid() -> Vec<SimDriver> {
        (0..12)
            .map(|index| {
                build_driver(
                    &format!("{:03}", index + 1),
                    60.0 + index as f64,
                    60.0,
                    65.0,
                    70.0,
                    8.0,
                )
            })
            .collect()
    }

    #[test]
    fn test_race_returns_all_drivers() {
        let grid = build_grid();
        let mut rng = StdRng::seed_from_u64(21);
        let ctx = sample_context(30, WeatherCondition::Dry);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

        assert_eq!(result.race_results.len(), 12);
    }

    #[test]
    fn test_race_positions_sequential() {
        let grid = build_grid();
        let mut rng = StdRng::seed_from_u64(22);
        let ctx = sample_context(30, WeatherCondition::Dry);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);
        assert_eq!(
            result
                .race_results
                .iter()
                .map(|value| value.finish_position)
                .collect::<Vec<_>>(),
            (1..=12).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_race_tire_degradation() {
        let grid = build_grid();
        let mut rng = StdRng::seed_from_u64(23);
        let ctx = sample_context(45, WeatherCondition::Dry);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

        assert!(result
            .race_results
            .iter()
            .all(|driver| driver.final_tire_wear < 1.0));
    }

    #[test]
    fn test_race_physical_degradation() {
        let grid = build_grid();
        let mut rng = StdRng::seed_from_u64(24);
        let ctx = sample_context(45, WeatherCondition::Dry);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

        assert!(result
            .race_results
            .iter()
            .all(|driver| driver.final_physical < 1.0));
    }

    #[test]
    fn test_race_positions_gained_calculated() {
        let grid = build_grid();
        let mut rng = StdRng::seed_from_u64(25);
        let ctx = sample_context(30, WeatherCondition::Dry);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

        assert!(
            result
                .race_results
                .iter()
                .all(|driver| driver.positions_gained
                    == driver.grid_position - driver.finish_position)
        );
    }

    #[test]
    fn test_race_good_driver_tends_to_win() {
        let ace = build_driver("ACE", 95.0, 90.0, 85.0, 88.0, 15.0);
        let grid: Vec<SimDriver> = std::iter::once(ace.clone())
            .chain(
                (0..11)
                    .map(|index| build_driver(&format!("R{index}"), 60.0, 60.0, 60.0, 60.0, 6.0)),
            )
            .collect();

        let mut wins = 0;
        for seed in 0..50 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ctx = sample_context(35, WeatherCondition::Dry);
            let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
            let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);
            if result.winner_id == ace.id {
                wins += 1;
            }
        }

        assert!(wins >= 35, "ace driver only won {} times", wins);
    }

    #[test]
    fn test_race_bad_tires_hurt_late_segments() {
        let tire_saver = build_driver("SAVE", 78.0, 72.0, 92.0, 75.0, 10.0);
        let tire_abuser = build_driver("ABUSE", 78.0, 72.0, 25.0, 75.0, 10.0);
        let grid = vec![tire_saver.clone(), tire_abuser.clone()];

        let mut saver_better = 0;
        for seed in 0..30 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ctx = sample_context(60, WeatherCondition::Dry);
            let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
            let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);
            if result.winner_id == tire_saver.id {
                saver_better += 1;
            }
        }

        assert!(saver_better >= 20);
    }

    #[test]
    fn test_incidents_can_generate_dnfs_when_enabled() {
        let mut risky = build_driver("RISK", 65.0, 30.0, 50.0, 55.0, 5.0);
        risky.consistencia = 20;
        risky.aggression = 95;
        risky.experiencia = 10;
        risky.car_reliability = 20.0;

        let grid = vec![risky.clone()];
        let mut found_dnf = false;

        for seed in 0..300 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ctx = sample_context_with_incidents(50, WeatherCondition::HeavyRain);
            let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
            let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

            if result.race_results.iter().any(|driver| driver.is_dnf) {
                found_dnf = true;
                break;
            }
        }

        assert!(
            found_dnf,
            "expected at least one DNF with incidents enabled"
        );
    }

    #[test]
    fn test_race_result_tracks_total_incidents() {
        let mut risky = build_driver("RISK", 65.0, 30.0, 50.0, 55.0, 5.0);
        risky.consistencia = 25;
        risky.aggression = 90;
        risky.experiencia = 15;
        risky.car_reliability = 25.0;

        let grid = vec![risky];
        let mut rng = StdRng::seed_from_u64(999);
        let ctx = sample_context_with_incidents(45, WeatherCondition::Wet);
        let qualifying = simulate_qualifying(&grid, &ctx, &mut rng);
        let result = simulate_race(&grid, &qualifying, &ctx, &mut rng);

        let sum: i32 = result
            .race_results
            .iter()
            .map(|driver| driver.incidents_count)
            .sum();
        assert_eq!(result.total_incidents, sum);
    }
}
