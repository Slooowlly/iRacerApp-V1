use crate::calendar::CalendarEntry;
use crate::constants::categories::get_category_config;
use crate::models::team::Team;
use crate::simulation::car_build::{
    profile_budget_cost, track_advantage, CarBuildProfile,
};
use crate::simulation::track_profile::get_track_simulation_data;

const ALL_PROFILES: [CarBuildProfile; 7] = [
    CarBuildProfile::Balanced,
    CarBuildProfile::AccelerationIntermediate,
    CarBuildProfile::PowerIntermediate,
    CarBuildProfile::HandlingIntermediate,
    CarBuildProfile::AccelerationExtreme,
    CarBuildProfile::PowerExtreme,
    CarBuildProfile::HandlingExtreme,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompetitiveOutlook {
    TitleFavorite,
    PromotionPush,
    Safe,
    RelegationRisk,
}

pub fn choose_car_build_profile(
    team: &Team,
    category_peers: &[Team],
    calendar: &[CalendarEntry],
) -> CarBuildProfile {
    let outlook = classify_outlook(team, category_peers);
    let mut best_profile = CarBuildProfile::Balanced;
    let mut best_score = f64::NEG_INFINITY;

    for profile in ALL_PROFILES {
        let score = score_profile(team, category_peers, calendar, outlook, profile);
        if score > best_score {
            best_score = score;
            best_profile = profile;
        }
    }

    best_profile
}

fn score_profile(
    team: &Team,
    category_peers: &[Team],
    calendar: &[CalendarEntry],
    outlook: CompetitiveOutlook,
    profile: CarBuildProfile,
) -> f64 {
    calendar_fit_score(calendar, profile)
        + strategy_bias(outlook, profile)
        + budget_bias(team.budget, profile)
        + car_strength_bias(team, category_peers, profile)
        + movement_bias(team, profile)
}

fn calendar_fit_score(calendar: &[CalendarEntry], profile: CarBuildProfile) -> f64 {
    calendar
        .iter()
        .map(|entry| {
            let data = get_track_simulation_data(entry.track_id);
            track_advantage(
                profile,
                (
                    data.acceleration_weight,
                    data.power_weight,
                    data.handling_weight,
                ),
            )
        })
        .sum()
}

fn strategy_bias(outlook: CompetitiveOutlook, profile: CarBuildProfile) -> f64 {
    match outlook {
        CompetitiveOutlook::TitleFavorite => match profile {
            CarBuildProfile::Balanced => 10.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 3.0,
            _ => -7.0,
        },
        CompetitiveOutlook::PromotionPush => match profile {
            CarBuildProfile::Balanced => 4.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 5.0,
            _ => -1.0,
        },
        CompetitiveOutlook::Safe => match profile {
            CarBuildProfile::Balanced => 5.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 2.0,
            _ => -4.0,
        },
        CompetitiveOutlook::RelegationRisk => match profile {
            CarBuildProfile::Balanced => -6.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 4.0,
            _ => 8.0,
        },
    }
}

fn budget_bias(budget: f64, profile: CarBuildProfile) -> f64 {
    let remaining = budget - profile_budget_cost(profile);
    if remaining < 0.0 {
        return -40.0;
    }

    let mut score = remaining / 8.0;

    if budget < 30.0 {
        score += match profile {
            CarBuildProfile::Balanced => -12.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => -4.0,
            _ => 4.0,
        };
    } else if budget < 50.0 {
        score += match profile {
            CarBuildProfile::Balanced => -6.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 0.0,
            _ => 2.0,
        };
    } else if budget > 75.0 {
        score += match profile {
            CarBuildProfile::Balanced => 4.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 1.0,
            _ => -2.0,
        };
    }

    score
}

fn car_strength_bias(team: &Team, category_peers: &[Team], profile: CarBuildProfile) -> f64 {
    let percentile = performance_percentile(team, category_peers);
    if percentile <= 0.20 {
        match profile {
            CarBuildProfile::Balanced => 4.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 1.0,
            _ => -5.0,
        }
    } else if percentile >= 0.75 {
        match profile {
            CarBuildProfile::Balanced => -4.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 2.0,
            _ => 5.0,
        }
    } else {
        0.0
    }
}

fn movement_bias(team: &Team, profile: CarBuildProfile) -> f64 {
    let Some(previous_category) = team.categoria_anterior.as_deref() else {
        return 0.0;
    };
    let Some(previous) = get_category_config(previous_category) else {
        return 0.0;
    };
    let Some(current) = get_category_config(&team.categoria) else {
        return 0.0;
    };

    if current.tier > previous.tier {
        match profile {
            CarBuildProfile::Balanced => 5.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 2.0,
            _ => -4.0,
        }
    } else if current.tier < previous.tier {
        match profile {
            CarBuildProfile::Balanced => 2.0,
            CarBuildProfile::AccelerationIntermediate
            | CarBuildProfile::PowerIntermediate
            | CarBuildProfile::HandlingIntermediate => 3.0,
            _ => 1.0,
        }
    } else {
        0.0
    }
}

fn classify_outlook(team: &Team, category_peers: &[Team]) -> CompetitiveOutlook {
    let percentile = performance_percentile(team, category_peers);
    let tier = get_category_config(&team.categoria).map(|config| config.tier).unwrap_or(0);

    if percentile <= 0.20 {
        CompetitiveOutlook::TitleFavorite
    } else if tier < 4 && percentile <= 0.40 {
        CompetitiveOutlook::PromotionPush
    } else if percentile >= 0.75 {
        CompetitiveOutlook::RelegationRisk
    } else {
        CompetitiveOutlook::Safe
    }
}

fn performance_percentile(team: &Team, category_peers: &[Team]) -> f64 {
    if category_peers.len() <= 1 {
        return 0.0;
    }

    let mut ordered = category_peers.to_vec();
    ordered.sort_by(|a, b| b.car_performance.total_cmp(&a.car_performance));
    let index = ordered
        .iter()
        .position(|candidate| candidate.id == team.id)
        .unwrap_or(0);
    index as f64 / (ordered.len() - 1) as f64
}

#[cfg(test)]
mod tests {
    use crate::calendar::CalendarEntry;
    use crate::models::enums::{RaceStatus, SeasonPhase, ThematicSlot, WeatherCondition};
    use crate::models::team::placeholder_team_from_db;

    use super::*;

    fn sample_team(id: &str, category: &str, car: f64, budget: f64) -> Team {
        let mut team = placeholder_team_from_db(
            id.to_string(),
            format!("Team {id}"),
            category.to_string(),
            "2026-01-01T00:00:00".to_string(),
        );
        team.car_performance = car;
        team.budget = budget;
        team
    }

    fn sample_calendar_entry(id: &str, season_id: &str, category: &str, rodada: i32, track_id: u32) -> CalendarEntry {
        CalendarEntry {
            id: id.to_string(),
            season_id: season_id.to_string(),
            categoria: category.to_string(),
            rodada,
            nome: format!("Round {rodada}"),
            track_id,
            track_name: format!("Track {track_id}"),
            track_config: "Full".to_string(),
            clima: WeatherCondition::Dry,
            temperatura: 22.0,
            voltas: 20,
            duracao_corrida_min: 30,
            duracao_classificacao_min: 15,
            status: RaceStatus::Pendente,
            horario: "14:00".to_string(),
            week_of_year: rodada,
            season_phase: SeasonPhase::BlocoRegular,
            display_date: "2026-01-01".to_string(),
            thematic_slot: ThematicSlot::NaoClassificado,
        }
    }

    #[test]
    fn strong_rich_team_prefers_balanced_on_mixed_calendar() {
        let target = sample_team("T001", "gt4", 12.0, 85.0);
        let peers = vec![
            target.clone(),
            sample_team("T002", "gt4", 10.0, 65.0),
            sample_team("T003", "gt4", 8.0, 55.0),
            sample_team("T004", "gt4", 6.0, 40.0),
        ];
        let calendar = vec![
            sample_calendar_entry("R1", "S002", "gt4", 1, 238),
            sample_calendar_entry("R2", "S002", "gt4", 2, 212),
            sample_calendar_entry("R3", "S002", "gt4", 3, 455),
            sample_calendar_entry("R4", "S002", "gt4", 4, 164),
        ];

        let profile = choose_car_build_profile(&target, &peers, &calendar);
        assert_eq!(profile, CarBuildProfile::Balanced);
    }

    #[test]
    fn poor_bottom_team_prefers_power_specialization_on_power_calendar() {
        let target = sample_team("T004", "gt4", 4.0, 18.0);
        let peers = vec![
            sample_team("T001", "gt4", 12.0, 85.0),
            sample_team("T002", "gt4", 10.0, 65.0),
            sample_team("T003", "gt4", 8.0, 55.0),
            target.clone(),
        ];
        let calendar = vec![
            sample_calendar_entry("R1", "S002", "gt4", 1, 93),
            sample_calendar_entry("R2", "S002", "gt4", 2, 287),
            sample_calendar_entry("R3", "S002", "gt4", 3, 188),
            sample_calendar_entry("R4", "S002", "gt4", 4, 397),
        ];

        let profile = choose_car_build_profile(&target, &peers, &calendar);
        assert!(matches!(
            profile,
            CarBuildProfile::PowerIntermediate | CarBuildProfile::PowerExtreme
        ));
    }
}
