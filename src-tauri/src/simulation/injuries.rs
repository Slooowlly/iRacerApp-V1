use rand::Rng;
use uuid::Uuid;

use crate::models::enums::InjuryType;
use crate::models::injury::Injury;
use crate::simulation::incidents::{IncidentResult, IncidentSeverity};

/// Generates a persistent Injury from a simulated incident.
/// Returns None if the incident is not severe enough or if the driver gets lucky.
pub fn generate_injury_from_incident(
    incident: &IncidentResult,
    season: i32,
    race_id: &str,
    rng: &mut impl Rng,
) -> Option<Injury> {
    // Only Critical incidents can cause injuries
    if incident.severity != IncidentSeverity::Critical {
        return None;
    }

    // 40% chance of injury
    if rng.gen_bool(0.4) {
        let roll = rng.gen_range(1..=100);
        let (injury_type, modifier, races_total, skill_penalty) = if roll <= 60 {
            // 60% Leve
            (InjuryType::Leve, 0.95, 2, 0.05)
        } else if roll <= 90 {
            // 30% Moderada
            (InjuryType::Moderada, 0.88, 4, 0.10)
        } else {
            // 10% Grave
            (InjuryType::Grave, 0.75, 8, 0.15)
        };

        Some(Injury {
            id: Uuid::new_v4().to_string(),
            pilot_id: incident.pilot_id.clone(),
            injury_type,
            modifier,
            races_total,
            races_remaining: races_total,
            skill_penalty,
            season,
            race_occurred: race_id.to_string(),
            active: true,
        })
    } else {
        None
    }
}
