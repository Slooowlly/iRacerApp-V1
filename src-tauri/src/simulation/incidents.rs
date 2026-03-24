use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::models::enums::WeatherCondition;

use super::context::SimDriver;
use super::race::{RaceSegment, RaceState};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IncidentType {
    Mechanical,
    DriverError,
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IncidentSeverity {
    Minor,
    Major,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentResult {
    pub pilot_id: String,
    pub incident_type: IncidentType,
    pub severity: IncidentSeverity,
    pub segment: String,
    pub positions_lost: i32,
    pub is_dnf: bool,
    pub description: String,
    #[serde(default)]
    pub linked_pilot_id: Option<String>,
}

const MECHANICAL_BASE_CHANCE: f64 = 0.015;
const DRIVER_ERROR_BASE_CHANCE: f64 = 0.017;
const COLLISION_BASE_CHANCE: f64 = 0.006;
const SEGMENTS: f64 = 5.0;

fn mechanical_segment_mult(segment: RaceSegment) -> f64 {
    match segment {
        RaceSegment::Start => 0.5,
        RaceSegment::Early => 0.8,
        RaceSegment::Mid => 1.0,
        RaceSegment::Late => 1.2,
        RaceSegment::Finish => 1.5,
    }
}

fn driver_error_segment_mult(segment: RaceSegment) -> f64 {
    match segment {
        RaceSegment::Start => 1.5,
        RaceSegment::Early => 1.0,
        RaceSegment::Mid => 1.0,
        RaceSegment::Late => 1.2,
        RaceSegment::Finish => 1.5,
    }
}

fn collision_segment_mult(segment: RaceSegment) -> f64 {
    match segment {
        RaceSegment::Start => 2.5,
        RaceSegment::Early => 1.0,
        RaceSegment::Mid => 0.8,
        RaceSegment::Late => 0.8,
        RaceSegment::Finish => 1.2,
    }
}

fn rain_base(weather: WeatherCondition) -> f64 {
    match weather {
        WeatherCondition::Dry => 0.0,
        WeatherCondition::Damp => 0.30,
        WeatherCondition::Wet => 0.60,
        WeatherCondition::HeavyRain => 1.00,
    }
}

fn rain_collision_mult(weather: WeatherCondition) -> f64 {
    match weather {
        WeatherCondition::Dry => 1.0,
        WeatherCondition::Damp => 1.2,
        WeatherCondition::Wet => 1.4,
        WeatherCondition::HeavyRain => 1.6,
    }
}

fn roll_mechanical(
    car_reliability: f64,
    segment: RaceSegment,
    rng: &mut impl Rng,
) -> Option<(IncidentSeverity, bool, i32)> {
    let base = MECHANICAL_BASE_CHANCE / SEGMENTS;
    let reliability_mod = (1.0 - ((car_reliability - 70.0) / 25.0 * 0.70)).clamp(0.1, 3.0);
    let chance = base * reliability_mod * mechanical_segment_mult(segment);

    if rng.gen::<f64>() >= chance {
        return None;
    }

    if rng.gen::<f64>() < 0.15 {
        Some((IncidentSeverity::Minor, false, rng.gen_range(1..=4)))
    } else {
        Some((IncidentSeverity::Major, true, 0))
    }
}

fn roll_driver_error(
    driver: &SimDriver,
    state: &RaceState,
    segment: RaceSegment,
    weather: WeatherCondition,
    is_championship_deciding: bool,
    rng: &mut impl Rng,
) -> Option<(IncidentSeverity, bool, i32)> {
    let base = DRIVER_ERROR_BASE_CHANCE / SEGMENTS;

    let consistency_core = (1.0 - driver.consistencia as f64 / 100.0).max(0.05);
    let aggression_core = 1.0 + driver.aggression as f64 / 200.0;
    let experience_mod = 1.0 - driver.experiencia as f64 / 100.0 * 0.30;

    let rb = rain_base(weather);
    let rain_absorption = driver.fator_chuva as f64 / 100.0 * 0.80;
    let rain_penalty = rb * (1.0 - rain_absorption);

    let pressure_mod = if is_championship_deciding {
        1.3 - driver.mentalidade as f64 / 100.0 * 0.25
    } else {
        1.0
    };

    let tire_mod = 1.0 + (1.0 - state.tire_wear) * 0.5;
    let fatigue_mod = 1.0 + (1.0 - state.physical_condition) * 0.4;

    let chance = (base
        * consistency_core
        * aggression_core
        * experience_mod
        * (1.0 + rain_penalty)
        * pressure_mod
        * tire_mod
        * fatigue_mod
        * driver_error_segment_mult(segment))
    .min(0.25);

    if rng.gen::<f64>() >= chance {
        return None;
    }

    if rng.gen::<f64>() < 0.70 {
        Some((IncidentSeverity::Minor, false, rng.gen_range(1..=4)))
    } else {
        Some((IncidentSeverity::Major, true, 0))
    }
}

fn roll_collision(
    driver: &SimDriver,
    position: i32,
    total_drivers: i32,
    avg_neighbor_aggression: f64,
    segment: RaceSegment,
    weather: WeatherCondition,
    rng: &mut impl Rng,
) -> Option<IncidentSeverity> {
    let base = COLLISION_BASE_CHANCE / SEGMENTS;

    let aggression_mod = 1.0 + driver.aggression as f64 / 100.0 * 0.60;
    let racecraft_mod = 1.0 - driver.racecraft as f64 / 100.0 * 0.50;
    let nearby_mod = 1.0 + avg_neighbor_aggression / 100.0 * 0.30;

    let pct = position as f64 / total_drivers.max(1) as f64;
    let pack_mod = if pct <= 0.25 {
        0.7
    } else if pct <= 0.75 {
        1.2
    } else {
        0.9
    };

    let chance = (base
        * aggression_mod
        * racecraft_mod
        * nearby_mod
        * pack_mod
        * rain_collision_mult(weather)
        * collision_segment_mult(segment))
    .min(0.20);

    if rng.gen::<f64>() >= chance {
        return None;
    }

    let roll = rng.gen::<f64>();
    if roll < 0.55 {
        Some(IncidentSeverity::Minor)
    } else if roll < 0.95 {
        Some(IncidentSeverity::Major)
    } else {
        Some(IncidentSeverity::Critical)
    }
}

fn resolve_collision_consequence(rng: &mut impl Rng) -> (bool, i32) {
    let roll = rng.gen::<f64>();
    if roll < 0.40 {
        (true, 0)
    } else if roll < 0.70 {
        (false, rng.gen_range(3..=5))
    } else {
        (false, rng.gen_range(1..=2))
    }
}

fn avg_neighbor_aggression(
    driver_id: &str,
    position: i32,
    drivers: &[SimDriver],
    states: &[RaceState],
) -> f64 {
    let mut total = 0.0;
    let mut count = 0;

    for state in states {
        if state.is_dnf || state.driver_id == driver_id {
            continue;
        }
        if (state.current_position - position).abs() <= 2 {
            if let Some(neighbor) = drivers.iter().find(|d| d.id == state.driver_id) {
                total += neighbor.aggression as f64;
                count += 1;
            }
        }
    }

    if count > 0 {
        total / count as f64
    } else {
        50.0
    }
}

fn find_neighbor(
    driver_id: &str,
    position: i32,
    states: &[RaceState],
    excluded: &[String],
) -> Option<String> {
    for target_pos in [position + 1, position - 1] {
        if let Some(state) = states.iter().find(|s| {
            s.current_position == target_pos
                && !s.is_dnf
                && s.driver_id != driver_id
                && !excluded.contains(&s.driver_id)
        }) {
            return Some(state.driver_id.clone());
        }
    }
    None
}

fn make_incident(
    pilot_id: String,
    incident_type: IncidentType,
    severity: IncidentSeverity,
    segment: &str,
    positions_lost: i32,
    is_dnf: bool,
    description: String,
    linked_pilot_id: Option<String>,
) -> IncidentResult {
    IncidentResult {
        pilot_id,
        incident_type,
        severity,
        segment: segment.to_string(),
        positions_lost,
        is_dnf,
        description,
        linked_pilot_id,
    }
}

pub fn process_segment_incidents(
    drivers: &[SimDriver],
    states: &[RaceState],
    segment: RaceSegment,
    weather: WeatherCondition,
    is_championship_deciding: bool,
    rng: &mut impl Rng,
) -> Vec<IncidentResult> {
    let mut incidents = Vec::new();
    let total_drivers = states.len() as i32;
    let seg = segment.as_str();
    let mut affected: Vec<String> = Vec::new();

    for state in states {
        if state.is_dnf || affected.contains(&state.driver_id) {
            continue;
        }

        let Some(driver) = drivers.iter().find(|d| d.id == state.driver_id) else {
            continue;
        };

        if let Some((severity, is_dnf, pos_lost)) =
            roll_mechanical(driver.car_reliability, segment, rng)
        {
            let desc = if is_dnf {
                format!("{} abandona com problema mecanico", driver.nome)
            } else {
                format!(
                    "{} perde {} posicoes por problema mecanico",
                    driver.nome, pos_lost
                )
            };
            incidents.push(make_incident(
                driver.id.clone(),
                IncidentType::Mechanical,
                severity,
                seg,
                pos_lost,
                is_dnf,
                desc,
                None,
            ));
            affected.push(driver.id.clone());
            continue;
        }

        if let Some((severity, is_dnf, pos_lost)) = roll_driver_error(
            driver,
            state,
            segment,
            weather,
            is_championship_deciding,
            rng,
        ) {
            let desc = if is_dnf {
                format!("{} abandona apos erro de pilotagem", driver.nome)
            } else {
                format!("{} comete erro e perde {} posicoes", driver.nome, pos_lost)
            };
            incidents.push(make_incident(
                driver.id.clone(),
                IncidentType::DriverError,
                severity,
                seg,
                pos_lost,
                is_dnf,
                desc,
                None,
            ));
            affected.push(driver.id.clone());
            continue;
        }

        let neighbor_agg =
            avg_neighbor_aggression(&driver.id, state.current_position, drivers, states);

        if let Some(severity) = roll_collision(
            driver,
            state.current_position,
            total_drivers,
            neighbor_agg,
            segment,
            weather,
            rng,
        ) {
            let neighbor_id = find_neighbor(&driver.id, state.current_position, states, &affected);
            let (trig_dnf, trig_lost) = resolve_collision_consequence(rng);
            let trig_desc = if trig_dnf {
                format!("{} abandona apos colisao", driver.nome)
            } else {
                format!("{} perde {} posicoes em colisao", driver.nome, trig_lost)
            };
            incidents.push(make_incident(
                driver.id.clone(),
                IncidentType::Collision,
                severity,
                seg,
                trig_lost,
                trig_dnf,
                trig_desc,
                neighbor_id.clone(),
            ));
            affected.push(driver.id.clone());

            if let Some(neighbor_id) = neighbor_id
            {
                let neighbor_name = drivers
                    .iter()
                    .find(|d| d.id == neighbor_id)
                    .map(|d| d.nome.as_str())
                    .unwrap_or("Piloto");
                let (nb_dnf, nb_lost) = resolve_collision_consequence(rng);
                let nb_desc = if nb_dnf {
                    format!(
                        "{} abandona apos colisao com {}",
                        neighbor_name, driver.nome
                    )
                } else {
                    format!(
                        "{} perde {} posicoes em colisao com {}",
                        neighbor_name, nb_lost, driver.nome
                    )
                };
                incidents.push(make_incident(
                    neighbor_id.clone(),
                    IncidentType::Collision,
                    severity,
                    seg,
                    nb_lost,
                    nb_dnf,
                    nb_desc,
                    Some(driver.id.clone()),
                ));
                affected.push(neighbor_id);
            }
        }
    }

    incidents
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    fn make_driver(
        id: &str,
        consistency: u8,
        aggression: u8,
        racecraft: u8,
        reliability: f64,
    ) -> SimDriver {
        SimDriver {
            id: id.to_string(),
            nome: format!("Driver {id}"),
            is_jogador: false,
            skill: 70,
            consistencia: consistency,
            racecraft,
            defesa: 50,
            ritmo_classificacao: 70,
            gestao_pneus: 60,
            habilidade_largada: 60,
            adaptabilidade: 50,
            fator_chuva: 50,
            fitness: 70,
            experiencia: 50,
            aggression,
            smoothness: 50,
            mentalidade: 60,
            confianca: 60,
            car_performance: 8.0,
            car_reliability: reliability,
            team_id: format!("T{id}"),
            team_name: format!("Team {id}"),
            corridas_na_categoria: 10,
        }
    }

    fn make_state(id: &str, position: i32) -> RaceState {
        RaceState {
            driver_id: id.to_string(),
            tire_wear: 1.0,
            physical_condition: 1.0,
            cumulative_score: 100.0 - position as f64 * 5.0,
            is_dnf: false,
            current_position: position,
            incidents: Vec::new(),
            dnf_reason: None,
            dnf_segment: None,
        }
    }

    #[test]
    fn test_safe_driver_rarely_has_incidents() {
        let drivers = vec![make_driver("P1", 95, 30, 85, 95.0)];
        let states = vec![make_state("P1", 1)];
        let mut rng = StdRng::seed_from_u64(42);

        let mut total = 0;
        for _ in 0..200 {
            let inc = process_segment_incidents(
                &drivers,
                &states,
                RaceSegment::Mid,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            total += inc.len();
        }

        assert!(
            total < 20,
            "safe driver had {total} incidents in 200 segments"
        );
    }

    #[test]
    fn test_unreliable_car_has_more_mechanicals() {
        let good = make_driver("G", 70, 50, 70, 95.0);
        let bad = make_driver("B", 70, 50, 70, 30.0);
        let mut rng = StdRng::seed_from_u64(123);

        let (mut good_mech, mut bad_mech) = (0, 0);
        for _ in 0..1000 {
            let inc = process_segment_incidents(
                &[good.clone()],
                &[make_state("G", 1)],
                RaceSegment::Mid,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            good_mech += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::Mechanical)
                .count();

            let inc = process_segment_incidents(
                &[bad.clone()],
                &[make_state("B", 1)],
                RaceSegment::Mid,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            bad_mech += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::Mechanical)
                .count();
        }

        assert!(
            bad_mech > good_mech,
            "bad={bad_mech} should > good={good_mech}"
        );
    }

    #[test]
    fn test_rain_increases_driver_errors() {
        let driver = make_driver("P1", 60, 50, 70, 80.0);
        let mut rng = StdRng::seed_from_u64(456);

        let (mut dry_err, mut wet_err) = (0, 0);
        for _ in 0..1000 {
            let state = make_state("P1", 5);
            let inc = process_segment_incidents(
                &[driver.clone()],
                &[state.clone()],
                RaceSegment::Mid,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            dry_err += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::DriverError)
                .count();

            let inc = process_segment_incidents(
                &[driver.clone()],
                &[state],
                RaceSegment::Mid,
                WeatherCondition::HeavyRain,
                false,
                &mut rng,
            );
            wet_err += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::DriverError)
                .count();
        }

        assert!(wet_err > dry_err, "wet={wet_err} should > dry={dry_err}");
    }

    #[test]
    fn test_collision_can_involve_neighbor() {
        let drivers: Vec<_> = (1..=6)
            .map(|i| make_driver(&format!("P{i}"), 50, 90, 30, 80.0))
            .collect();
        let states: Vec<_> = (1..=6).map(|i| make_state(&format!("P{i}"), i)).collect();
        let mut rng = StdRng::seed_from_u64(789);

        let mut pairs = 0;
        for _ in 0..500 {
            let inc = process_segment_incidents(
                &drivers,
                &states,
                RaceSegment::Start,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            let collisions = inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::Collision)
                .count();
            if collisions >= 2 {
                pairs += 1;
            }
        }

        assert!(pairs > 0, "should produce collision pairs");
    }

    #[test]
    fn test_dnf_driver_not_processed() {
        let drivers = vec![make_driver("P1", 30, 90, 30, 20.0)];
        let mut state = make_state("P1", 1);
        state.is_dnf = true;

        let mut rng = StdRng::seed_from_u64(111);
        let inc = process_segment_incidents(
            &drivers,
            &[state],
            RaceSegment::Start,
            WeatherCondition::HeavyRain,
            true,
            &mut rng,
        );
        assert!(inc.is_empty());
    }

    #[test]
    fn test_start_segment_more_collisions_than_mid() {
        let drivers: Vec<_> = (1..=12)
            .map(|i| make_driver(&format!("P{i}"), 60, 65, 55, 80.0))
            .collect();
        let states: Vec<_> = (1..=12).map(|i| make_state(&format!("P{i}"), i)).collect();
        let mut rng = StdRng::seed_from_u64(333);

        let (mut start_c, mut mid_c) = (0, 0);
        for _ in 0..500 {
            let inc = process_segment_incidents(
                &drivers,
                &states,
                RaceSegment::Start,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            start_c += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::Collision)
                .count();

            let inc = process_segment_incidents(
                &drivers,
                &states,
                RaceSegment::Mid,
                WeatherCondition::Dry,
                false,
                &mut rng,
            );
            mid_c += inc
                .iter()
                .filter(|i| i.incident_type == IncidentType::Collision)
                .count();
        }

        assert!(start_c > mid_c, "start={start_c} should > mid={mid_c}");
    }

    #[test]
    fn test_one_incident_per_driver_per_segment() {
        let drivers: Vec<_> = (1..=8)
            .map(|i| make_driver(&format!("P{i}"), 40, 80, 30, 40.0))
            .collect();
        let states: Vec<_> = (1..=8).map(|i| make_state(&format!("P{i}"), i)).collect();
        let mut rng = StdRng::seed_from_u64(555);

        for _ in 0..200 {
            let inc = process_segment_incidents(
                &drivers,
                &states,
                RaceSegment::Start,
                WeatherCondition::Wet,
                true,
                &mut rng,
            );
            let mut seen = HashSet::new();
            for incident in &inc {
                assert!(
                    seen.insert(&incident.pilot_id),
                    "driver {} had duplicate incident",
                    incident.pilot_id
                );
            }
        }
    }
}
