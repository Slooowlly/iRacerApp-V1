use rand::Rng;

use crate::market::proposals::MarketProposal;
use crate::models::contract::Contract;
use crate::models::driver::Driver;
use crate::models::enums::{PrimaryPersonality, TeamRole};

#[derive(Debug, Clone)]
pub struct ProposalDecision {
    pub accepted: bool,
    pub reason: String,
}

pub fn evaluate_proposal(
    driver: &Driver,
    proposal: &MarketProposal,
    current_contract: Option<&Contract>,
    current_category_tier: u8,
    proposal_category_tier: u8,
    team_car_performance: f64,
    team_reputacao: f64,
    rng: &mut impl Rng,
) -> ProposalDecision {
    let car_perf_normalized = (((team_car_performance + 5.0) / 21.0) * 100.0).clamp(0.0, 100.0);
    let salary_minimum = calculate_salary_minimum(driver, current_category_tier);

    if proposal.salario_oferecido < salary_minimum * 0.80 {
        return reject("Salário abaixo do aceitável");
    }

    if proposal_category_tier < current_category_tier && current_contract.is_some() {
        if rng.gen_range(0.0..1.0) > 0.30 {
            return reject("Não quer descer de categoria");
        }
    }

    if proposal.papel == TeamRole::Numero2
        && driver.atributos.skill > 70.0
        && rng.gen_range(0.0..1.0) < 0.50
    {
        return reject("Quer ser N1");
    }

    if car_perf_normalized < 40.0 && rng.gen_range(0.0..1.0) < 0.60 {
        return reject("Equipe com carro fraco");
    }

    let mut score = (car_perf_normalized / 100.0) * 30.0
        + (proposal_category_tier as f64 / 5.0) * 25.0
        + if proposal.papel == TeamRole::Numero1 {
            15.0
        } else {
            10.0
        }
        + (proposal.salario_oferecido / 300_000.0).min(1.0) * 15.0
        + (team_reputacao / 100.0) * 10.0;

    match &driver.personalidade_primaria {
        Some(PrimaryPersonality::Ambicioso) => {
            if proposal_category_tier > current_category_tier {
                score += 15.0;
            }
            if proposal_category_tier <= current_category_tier {
                score -= 10.0;
            }
        }
        Some(PrimaryPersonality::Mercenario) => {
            if proposal.salario_oferecido > salary_minimum * 1.3 {
                score += 20.0;
            }
        }
        Some(PrimaryPersonality::Consolidador) => {
            if proposal_category_tier == current_category_tier && car_perf_normalized > 60.0 {
                score += 10.0;
            }
        }
        Some(PrimaryPersonality::Leal) => {
            score -= 15.0;
        }
        _ => {}
    }

    let threshold = 50.0 + rng.gen_range(-10.0..=10.0);
    if score >= threshold {
        accept()
    } else {
        reject("Proposta não atrativa o suficiente")
    }
}

fn calculate_salary_minimum(driver: &Driver, current_category_tier: u8) -> f64 {
    let tier_base = match current_category_tier {
        0 => 10_000.0,
        1 => 22_000.0,
        2 => 40_000.0,
        3 => 80_000.0,
        4 => 140_000.0,
        _ => 160_000.0,
    };

    let skill_factor = (driver.atributos.skill / 70.0).clamp(0.6, 1.6);
    (tier_base * skill_factor).max(5_000.0)
}

fn accept() -> ProposalDecision {
    ProposalDecision {
        accepted: true,
        reason: "Proposta aceita".to_string(),
    }
}

fn reject(reason: &str) -> ProposalDecision {
    ProposalDecision {
        accepted: false,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::market::proposals::{MarketProposal, ProposalStatus};

    #[test]
    fn test_accept_good_proposal() {
        let driver = sample_driver(65.0, None);
        let proposal = sample_proposal(120_000.0, TeamRole::Numero1);
        let mut rng = StdRng::seed_from_u64(1);

        let result = evaluate_proposal(&driver, &proposal, None, 2, 3, 12.0, 80.0, &mut rng);

        assert!(result.accepted);
    }

    #[test]
    fn test_reject_low_salary() {
        let driver = sample_driver(68.0, None);
        let proposal = sample_proposal(5_000.0, TeamRole::Numero1);
        let mut rng = StdRng::seed_from_u64(2);

        let result = evaluate_proposal(&driver, &proposal, None, 3, 3, 10.0, 70.0, &mut rng);

        assert!(!result.accepted);
        assert!(result.reason.contains("Salário"));
    }

    #[test]
    fn test_reject_category_downgrade() {
        let driver = sample_driver(72.0, None);
        let proposal = sample_proposal(90_000.0, TeamRole::Numero1);
        let contract = sample_contract();
        let mut rng = StdRng::seed_from_u64(7);

        let result = evaluate_proposal(
            &driver,
            &proposal,
            Some(&contract),
            4,
            2,
            10.0,
            70.0,
            &mut rng,
        );

        assert!(!result.accepted);
    }

    #[test]
    fn test_ambitious_accepts_promotion() {
        let driver = sample_driver(66.0, Some(PrimaryPersonality::Ambicioso));
        let proposal = sample_proposal(80_000.0, TeamRole::Numero2);
        let mut rng = StdRng::seed_from_u64(3);

        let result = evaluate_proposal(&driver, &proposal, None, 1, 3, 8.0, 70.0, &mut rng);

        assert!(result.accepted);
    }

    #[test]
    fn test_mercenary_follows_money() {
        let driver = sample_driver(64.0, Some(PrimaryPersonality::Mercenario));
        let proposal = sample_proposal(220_000.0, TeamRole::Numero2);
        let mut rng = StdRng::seed_from_u64(4);

        let result = evaluate_proposal(&driver, &proposal, None, 2, 2, 6.0, 55.0, &mut rng);

        assert!(result.accepted);
    }

    #[test]
    fn test_loyal_resists_transfer() {
        let driver = sample_driver(64.0, Some(PrimaryPersonality::Leal));
        let proposal = sample_proposal(80_000.0, TeamRole::Numero1);
        let contract = sample_contract();
        let mut rng = StdRng::seed_from_u64(5);

        let result = evaluate_proposal(
            &driver,
            &proposal,
            Some(&contract),
            2,
            2,
            8.0,
            60.0,
            &mut rng,
        );

        assert!(!result.accepted);
    }

    fn sample_driver(skill: f64, personality: Option<PrimaryPersonality>) -> Driver {
        let mut driver = Driver::new(
            "P001".to_string(),
            "Piloto".to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            25,
            2020,
        );
        driver.atributos.skill = skill;
        driver.personalidade_primaria = personality;
        driver
    }

    fn sample_proposal(salary: f64, role: TeamRole) -> MarketProposal {
        MarketProposal {
            id: "MP001".to_string(),
            equipe_id: "T001".to_string(),
            equipe_nome: "Equipe".to_string(),
            piloto_id: "P001".to_string(),
            piloto_nome: "Piloto".to_string(),
            categoria: "gt4".to_string(),
            papel: role,
            salario_oferecido: salary,
            duracao_anos: 2,
            status: ProposalStatus::Pendente,
            motivo_recusa: None,
        }
    }

    fn sample_contract() -> Contract {
        Contract::new(
            "C001".to_string(),
            "P001".to_string(),
            "Piloto".to_string(),
            "T001".to_string(),
            "Equipe Atual".to_string(),
            1,
            2,
            100_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        )
    }
}
