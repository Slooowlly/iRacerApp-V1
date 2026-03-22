use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::constants::scoring::{get_weather_penalty, QUALI_SCORE_TO_LAP_MS};
use crate::models::enums::WeatherCondition;

use super::context::{SimDriver, SimulationContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualifyingResult {
    pub pilot_id: String,
    pub pilot_name: String,
    pub team_id: String,
    pub team_name: String,
    pub position: i32,
    pub quali_score: f64,
    pub best_lap_time_ms: f64,
    pub gap_to_pole_ms: f64,
    pub is_pole: bool,
    pub is_jogador: bool,
}

pub fn simulate_qualifying(
    drivers: &[SimDriver],
    ctx: &SimulationContext,
    rng: &mut impl Rng,
) -> Vec<QualifyingResult> {
    let mut results: Vec<QualifyingResult> = drivers
        .iter()
        .map(|driver| {
            let mut score = driver.skill as f64 * 0.40
                + driver.ritmo_classificacao as f64 * 0.25
                + normalize_car_performance(driver.car_performance) * 0.25
                + driver.adaptabilidade as f64 * 0.10;

            score *= weather_multiplier(ctx.weather, driver.fator_chuva);

            if driver.corridas_na_categoria < 10 {
                let experience_penalty = (10 - driver.corridas_na_categoria) as f64 * 0.005;
                score *= 1.0 - experience_penalty;
            }

            let variance_range = (100.0 - driver.consistencia as f64) / 100.0 * 8.0;
            score += rng.gen_range(-variance_range..=variance_range);
            score = score.max(10.0);

            QualifyingResult {
                pilot_id: driver.id.clone(),
                pilot_name: driver.nome.clone(),
                team_id: driver.team_id.clone(),
                team_name: driver.team_name.clone(),
                position: 0,
                quali_score: score,
                best_lap_time_ms: 0.0,
                gap_to_pole_ms: 0.0,
                is_pole: false,
                is_jogador: driver.is_jogador,
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.quali_score
            .partial_cmp(&a.quali_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let pole_score = results
        .first()
        .map(|result| result.quali_score)
        .unwrap_or(0.0);
    let pole_time = ctx.base_lap_time_ms;

    for (index, result) in results.iter_mut().enumerate() {
        result.position = index as i32 + 1;
        result.is_pole = index == 0;
        result.best_lap_time_ms =
            ctx.base_lap_time_ms + (pole_score - result.quali_score).max(0.0) * QUALI_SCORE_TO_LAP_MS;
        result.gap_to_pole_ms = (result.best_lap_time_ms - pole_time).max(0.0);
    }

    results
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

    fn sample_context(weather: WeatherCondition) -> SimulationContext {
        SimulationContext {
            category_id: "mazda_rookie".to_string(),
            category_tier: 0,
            track_id: 1,
            track_name: "Laguna Seca".to_string(),
            weather,
            temperature: 22.0,
            total_laps: 12,
            race_duration_minutes: 20,
            is_championship_deciding: false,
            base_lap_time_ms: 90_000.0,
            tire_degradation_rate: 0.02,
            physical_degradation_rate: 0.01,
            incidents_enabled: false,
        }
    }

    fn build_driver(id: &str, skill: f64, quali: f64, rain: f64, consistency: f64) -> SimDriver {
        let mut driver = Driver::create_player(
            id.to_string(),
            format!("Driver {}", id),
            "🇧🇷 Brasileiro".to_string(),
            20,
        );
        driver.is_jogador = false;
        driver.atributos.skill = skill;
        driver.atributos.ritmo_classificacao = quali;
        driver.atributos.fator_chuva = rain;
        driver.atributos.consistencia = consistency;
        driver.atributos.adaptabilidade = 60.0;

        let mut team = placeholder_team_from_db(
            format!("T{}", id),
            format!("Team {}", id),
            "mazda_rookie".to_string(),
            "2026-01-01T00:00:00".to_string(),
        );
        team.car_performance = 8.0;

        SimDriver::from_driver_and_team(&driver, &team)
    }

    fn sample_grid() -> Vec<SimDriver> {
        (0..12)
            .map(|index| {
                build_driver(
                    &format!("{:03}", index + 1),
                    50.0 + index as f64,
                    48.0 + index as f64,
                    50.0,
                    85.0,
                )
            })
            .collect()
    }

    #[test]
    fn test_qualifying_returns_all_drivers() {
        let mut rng = StdRng::seed_from_u64(11);
        let results = simulate_qualifying(
            &sample_grid(),
            &sample_context(WeatherCondition::Dry),
            &mut rng,
        );
        assert_eq!(results.len(), 12);
    }

    #[test]
    fn test_qualifying_positions_sequential() {
        let mut rng = StdRng::seed_from_u64(12);
        let results = simulate_qualifying(
            &sample_grid(),
            &sample_context(WeatherCondition::Dry),
            &mut rng,
        );
        let positions: Vec<i32> = results.iter().map(|result| result.position).collect();
        assert_eq!(positions, (1..=12).collect::<Vec<_>>());
    }

    #[test]
    fn test_qualifying_higher_skill_tends_to_pole() {
        let elite = build_driver("A", 95.0, 95.0, 50.0, 90.0);
        let grid: Vec<SimDriver> = std::iter::once(elite.clone())
            .chain((0..11).map(|index| build_driver(&format!("B{index}"), 55.0, 55.0, 50.0, 75.0)))
            .collect();

        let mut top3_finishes = 0;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let results =
                simulate_qualifying(&grid, &sample_context(WeatherCondition::Dry), &mut rng);
            let pos = results
                .iter()
                .find(|result| result.pilot_id == elite.id)
                .expect("elite result")
                .position;
            if pos <= 3 {
                top3_finishes += 1;
            }
        }

        assert!(
            top3_finishes >= 80,
            "elite driver only reached top3 {} times",
            top3_finishes
        );
    }

    #[test]
    fn test_qualifying_rain_penalty_applied() {
        let wet_specialist = build_driver("RAIN", 75.0, 75.0, 95.0, 90.0);
        let wet_weak = build_driver("DRY", 75.0, 75.0, 20.0, 90.0);
        let grid = vec![wet_specialist.clone(), wet_weak.clone()];

        let mut better_in_rain = 0;
        for seed in 0..30 {
            let mut rng = StdRng::seed_from_u64(seed);
            let results = simulate_qualifying(
                &grid,
                &sample_context(WeatherCondition::HeavyRain),
                &mut rng,
            );
            if results[0].pilot_id == wet_specialist.id {
                better_in_rain += 1;
            }
        }

        assert!(better_in_rain >= 20);
    }

    #[test]
    fn test_qualifying_gap_to_pole() {
        let mut rng = StdRng::seed_from_u64(14);
        let results = simulate_qualifying(
            &sample_grid(),
            &sample_context(WeatherCondition::Dry),
            &mut rng,
        );
        assert_eq!(results[0].gap_to_pole_ms, 0.0);
        assert!(results
            .iter()
            .skip(1)
            .all(|result| result.gap_to_pole_ms > 0.0));
    }

    #[test]
    fn test_qualifying_lap_times_ordered() {
        let mut rng = StdRng::seed_from_u64(15);
        let results = simulate_qualifying(
            &sample_grid(),
            &sample_context(WeatherCondition::Dry),
            &mut rng,
        );
        assert!(results
            .windows(2)
            .all(|window| window[0].best_lap_time_ms <= window[1].best_lap_time_ms));
    }
}
