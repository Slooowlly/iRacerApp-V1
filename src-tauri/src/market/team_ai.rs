use rand::Rng;

use crate::constants::categories::get_category_config;
use crate::market::proposals::{MarketProposal, ProposalStatus, Vacancy};
use crate::models::driver::Driver;

#[derive(Debug, Clone)]
pub struct AvailableDriver {
    pub driver: Driver,
    pub visibility: f64,
    pub posicao_campeonato: i32,
    pub categoria_atual: String,
    pub category_tier: u8,
    /// Nível máximo de licença que o piloto possui. None = sem nenhuma licença.
    pub max_license_level: Option<u8>,
}

pub fn generate_team_proposals(
    vacancy: &Vacancy,
    available_drivers: &[AvailableDriver],
    current_season: i32,
    rng: &mut impl Rng,
) -> Vec<MarketProposal> {
    let required_license = get_category_config(&vacancy.categoria)
        .and_then(|config| config.licenca_necessaria);

    let mut candidates: Vec<&AvailableDriver> = available_drivers
        .iter()
        .filter(|available| {
            let license_ok = match required_license {
                None => true,
                Some(required) => available
                    .max_license_level
                    .map_or(false, |level| level >= required),
            };
            available.visibility >= 4.0
                && available.driver.status.as_str() != "Aposentado"
                && !available.driver.is_jogador
                && available.category_tier.abs_diff(vacancy.category_tier) <= 1
                && license_ok
        })
        .collect();

    candidates.sort_by(|a, b| candidate_score(b).total_cmp(&candidate_score(a)));

    candidates
        .into_iter()
        .take(3)
        .map(|candidate| MarketProposal {
            id: format!(
                "TMP-{}-{}-{}",
                vacancy.team_id, candidate.driver.id, current_season
            ),
            equipe_id: vacancy.team_id.clone(),
            equipe_nome: vacancy.team_name.clone(),
            piloto_id: candidate.driver.id.clone(),
            piloto_nome: candidate.driver.nome.clone(),
            categoria: vacancy.categoria.clone(),
            papel: vacancy.papel_necessario.clone(),
            salario_oferecido: calculate_offer_salary(vacancy, &candidate.driver, rng),
            duracao_anos: match vacancy.category_tier {
                0..=1 => rng.gen_range(1..=2),
                2..=3 => rng.gen_range(1..=3),
                _ => rng.gen_range(2..=3),
            },
            status: ProposalStatus::Pendente,
            motivo_recusa: None,
        })
        .collect()
}

fn candidate_score(available: &AvailableDriver) -> f64 {
    let age_bonus = if available.driver.idade < 24 {
        80.0
    } else if available.driver.idade <= 30 {
        100.0
    } else {
        50.0
    };

    available.driver.atributos.skill * 0.4
        + available.driver.atributos.consistencia * 0.2
        + (available.visibility * 10.0) * 0.2
        + age_bonus * 0.2
}

fn calculate_offer_salary(vacancy: &Vacancy, driver: &Driver, rng: &mut impl Rng) -> f64 {
    let tier_base = match vacancy.category_tier {
        0 => 10_000.0,
        1 => 25_000.0,
        2 => 50_000.0,
        3 => 100_000.0,
        4 => 200_000.0,
        _ => 150_000.0,
    };

    let skill_modifier = driver.atributos.skill / 70.0;
    let budget_modifier = (vacancy.budget / 70.0).min(1.5);
    let variance = rng.gen_range(0.85..=1.15);

    (tier_base * skill_modifier * budget_modifier * variance).max(5_000.0)
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::models::driver::Driver;
    use crate::models::enums::TeamRole;

    #[test]
    fn test_generate_proposals_for_vacancy() {
        let vacancy = sample_vacancy(3);
        let available = vec![
            sample_available_driver("P001", "gt4", 3, 6.5, 72.0),
            sample_available_driver("P002", "bmw_m2", 2, 5.5, 68.0),
            sample_available_driver("P003", "gt3", 4, 7.0, 74.0),
        ];
        let mut rng = StdRng::seed_from_u64(1);

        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert!(!proposals.is_empty());
        assert!(proposals.len() <= 3);
    }

    #[test]
    fn test_proposals_respect_tier_limit() {
        let vacancy = sample_vacancy(2);
        let available = vec![
            sample_available_driver("P001", "mazda_amador", 1, 6.0, 65.0),
            sample_available_driver("P002", "endurance", 5, 8.0, 82.0),
        ];
        let mut rng = StdRng::seed_from_u64(2);

        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert!(proposals
            .iter()
            .all(|proposal| proposal.piloto_id != "P002"));
    }

    #[test]
    fn test_proposals_salary_scales_with_tier() {
        let low = sample_vacancy(2);
        let high = sample_vacancy(3);
        let available = vec![sample_available_driver("P001", "gt4", 3, 7.0, 72.0)];
        let mut rng_low = StdRng::seed_from_u64(3);
        let mut rng_high = StdRng::seed_from_u64(3);

        let low_offer = generate_team_proposals(&low, &available, 2, &mut rng_low);
        let high_offer = generate_team_proposals(&high, &available, 2, &mut rng_high);

        assert!(high_offer[0].salario_oferecido > low_offer[0].salario_oferecido);
    }

    fn sample_vacancy(tier: u8) -> Vacancy {
        Vacancy {
            team_id: "T001".to_string(),
            team_name: "Equipe".to_string(),
            categoria: "gt4".to_string(),
            category_tier: tier,
            car_performance: 8.0,
            budget: 75.0,
            reputacao: 70.0,
            papel_necessario: TeamRole::Numero1,
            piloto_existente_id: None,
        }
    }

    fn sample_available_driver(
        id: &str,
        category: &str,
        tier: u8,
        visibility: f64,
        skill: f64,
    ) -> AvailableDriver {
        let mut driver = Driver::new(
            id.to_string(),
            format!("Piloto {id}"),
            "Brasil".to_string(),
            "M".to_string(),
            24,
            2020,
        );
        driver.atributos.skill = skill;
        driver.atributos.consistencia = 65.0;
        AvailableDriver {
            driver,
            visibility,
            posicao_campeonato: 3,
            categoria_atual: category.to_string(),
            category_tier: tier,
            // nível alto para não bloquear testes que não testam licença
            max_license_level: Some(10),
        }
    }
}
