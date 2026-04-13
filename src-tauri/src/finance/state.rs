use crate::models::team::Team;

pub fn derive_financial_state(score: f64) -> &'static str {
    match score {
        value if value >= 70.0 => "elite",
        value if value >= 55.0 => "healthy",
        value if value >= 40.0 => "stable",
        value if value >= 25.0 => "pressured",
        value if value >= 12.0 => "crisis",
        _ => "collapse",
    }
}

pub fn financial_health_score(team: &Team) -> f64 {
    let cash_score = (team.cash_balance / 75_000.0).clamp(-20.0, 100.0);
    let debt_penalty = (team.debt_balance / 200_000.0).clamp(0.0, 60.0);
    let structure_score = ((team.engineering + team.facilities) / 2.0).clamp(0.0, 100.0);
    let support_score = ((team.budget + team.reputacao) / 2.0).clamp(0.0, 100.0);
    let momentum_score =
        (team.last_round_net / 50_000.0).clamp(-15.0, 15.0) + team.stats_pontos as f64 * 0.05;

    (cash_score * 0.4 + structure_score * 0.25 + support_score * 0.2 + momentum_score * 0.15
        - debt_penalty)
        .clamp(0.0, 100.0)
}

pub fn choose_season_strategy(team: &Team) -> &'static str {
    if team.debt_balance >= 750_000.0 {
        return "survival";
    }

    if team.budget < 25.0 && team.car_performance < 6.0 {
        return "all_in";
    }

    match derive_financial_state(financial_health_score(team)) {
        "elite" => "balanced",
        "healthy" => {
            if team.car_performance < 8.0 {
                "expansion"
            } else {
                "balanced"
            }
        }
        "stable" => {
            if team.budget < 35.0 {
                "austerity"
            } else {
                "balanced"
            }
        }
        "pressured" => "all_in",
        "crisis" | "collapse" => "survival",
        _ => "balanced",
    }
}

pub fn refresh_team_financial_state(team: &mut Team) {
    team.financial_state = derive_financial_state(financial_health_score(team)).to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::team::placeholder_team_from_db;

    #[test]
    fn high_financial_health_maps_to_elite() {
        assert_eq!(derive_financial_state(90.0), "elite");
    }

    #[test]
    fn low_financial_health_maps_to_collapse() {
        assert_eq!(derive_financial_state(10.0), "collapse");
    }

    #[test]
    fn rich_structured_team_scores_as_elite() {
        let mut team = placeholder_team_from_db(
            "T001".to_string(),
            "Equipe Rica".to_string(),
            "gt3".to_string(),
            "2026-01-01".to_string(),
        );
        team.cash_balance = 8_000_000.0;
        team.debt_balance = 0.0;
        team.budget = 82.0;
        team.reputacao = 85.0;
        team.engineering = 88.0;
        team.facilities = 84.0;

        assert_eq!(
            derive_financial_state(financial_health_score(&team)),
            "elite"
        );
        assert_eq!(choose_season_strategy(&team), "balanced");
    }

    #[test]
    fn indebted_team_falls_into_survival_mode() {
        let mut team = placeholder_team_from_db(
            "T002".to_string(),
            "Equipe Quebrada".to_string(),
            "gt4".to_string(),
            "2026-01-01".to_string(),
        );
        team.cash_balance = -250_000.0;
        team.debt_balance = 1_200_000.0;
        team.budget = 18.0;
        team.engineering = 28.0;
        team.facilities = 24.0;
        team.last_round_net = -90_000.0;

        assert_eq!(
            derive_financial_state(financial_health_score(&team)),
            "collapse"
        );
        assert_eq!(choose_season_strategy(&team), "survival");
    }
}
