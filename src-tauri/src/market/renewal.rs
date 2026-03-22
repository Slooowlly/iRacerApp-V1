use rand::Rng;

use crate::models::contract::Contract;
use crate::models::driver::Driver;
use crate::models::enums::{PrimaryPersonality, TeamRole};

#[derive(Debug, Clone)]
pub struct RenewalDecision {
    pub should_renew: bool,
    pub reason: String,
    pub new_salary: Option<f64>,
    pub new_duration: Option<i32>,
    pub new_role: Option<TeamRole>,
}

pub fn should_renew_contract(
    driver: &Driver,
    performance_score: f64,
    contract: &Contract,
    team_budget: f64,
    rng: &mut impl Rng,
) -> RenewalDecision {
    let mut decision = if driver.idade > 36 && performance_score < 60.0 {
        no_renewal("Veterano com desempenho abaixo da média")
    } else if driver.idade > 36 && performance_score < 75.0 && rng.gen_range(0.0..1.0) < 0.50 {
        no_renewal("Veterano, equipe busca sangue novo")
    } else if performance_score < 35.0 {
        no_renewal("Desempenho muito fraco")
    } else if performance_score < 50.0 && rng.gen_range(0.0..1.0) < 0.60 {
        no_renewal("Desempenho abaixo da média")
    } else {
        let effective_budget = (team_budget.max(1.0) * 5_000.0).max(25_000.0);
        let salary_ratio = contract.salario_anual / effective_budget;
        if salary_ratio > 0.35 && performance_score < 70.0 {
            no_renewal("Salário desproporcional ao desempenho")
        } else if contract.papel == TeamRole::Numero2 && performance_score < 55.0 {
            no_renewal("N2 fraco, equipe busca jovem promessa")
        } else if contract.papel == TeamRole::Numero2
            && performance_score < 65.0
            && rng.gen_range(0.0..1.0) < 0.55
        {
            no_renewal("N2 mediano, chance de buscar melhor")
        } else {
            let new_salary = calculate_renewal_salary(contract, performance_score, driver);
            let new_duration = if performance_score > 80.0 {
                rng.gen_range(2..=3)
            } else if performance_score > 60.0 {
                rng.gen_range(1..=2)
            } else {
                1
            };
            RenewalDecision {
                should_renew: true,
                reason: "Desempenho satisfatório".into(),
                new_salary: Some(new_salary),
                new_duration: Some(new_duration),
                new_role: Some(contract.papel.clone()),
            }
        }
    };

    match &driver.personalidade_primaria {
        Some(PrimaryPersonality::Leal) => {
            if !decision.should_renew && performance_score > 40.0 {
                decision.should_renew = true;
                decision.reason = "Leal — equipe dá outra chance".into();
                decision.new_salary =
                    Some(calculate_renewal_salary(contract, performance_score, driver) * 0.90);
                decision.new_duration = Some(1);
                decision.new_role = Some(contract.papel.clone());
            } else if let Some(ref mut salary) = decision.new_salary {
                *salary *= 0.90;
            }
        }
        Some(PrimaryPersonality::Mercenario) => {
            if let Some(ref mut salary) = decision.new_salary {
                *salary *= 1.15;
            }
        }
        _ => {}
    }

    if let Some(ref mut salary) = decision.new_salary {
        *salary = salary.max(5_000.0).round();
    }

    decision
}

fn no_renewal(reason: &str) -> RenewalDecision {
    RenewalDecision {
        should_renew: false,
        reason: reason.to_string(),
        new_salary: None,
        new_duration: None,
        new_role: None,
    }
}

fn calculate_renewal_salary(contract: &Contract, performance: f64, driver: &Driver) -> f64 {
    let base = contract.salario_anual;
    let perf_modifier = if performance > 80.0 {
        1.20
    } else if performance > 60.0 {
        1.05
    } else {
        0.90
    };

    let age_modifier = if driver.idade > 34 { 0.85 } else { 1.0 };

    (base * perf_modifier * age_modifier).max(5_000.0)
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_renew_good_performer() {
        let driver = sample_driver(29, None);
        let contract = sample_contract(TeamRole::Numero1, 80_000.0);
        let mut rng = StdRng::seed_from_u64(1);

        let decision = should_renew_contract(&driver, 82.0, &contract, 90.0, &mut rng);

        assert!(decision.should_renew);
        assert!(decision.new_salary.is_some());
    }

    #[test]
    fn test_no_renew_bad_performer() {
        let driver = sample_driver(28, None);
        let contract = sample_contract(TeamRole::Numero1, 80_000.0);
        let mut rng = StdRng::seed_from_u64(2);

        let decision = should_renew_contract(&driver, 30.0, &contract, 90.0, &mut rng);

        assert!(!decision.should_renew);
    }

    #[test]
    fn test_no_renew_old_driver_low_performance() {
        let driver = sample_driver(38, None);
        let contract = sample_contract(TeamRole::Numero1, 80_000.0);
        let mut rng = StdRng::seed_from_u64(3);

        let decision = should_renew_contract(&driver, 58.0, &contract, 90.0, &mut rng);

        assert!(!decision.should_renew);
    }

    #[test]
    fn test_loyal_driver_easier_renewal() {
        let driver = sample_driver(31, Some(PrimaryPersonality::Leal));
        let contract = sample_contract(TeamRole::Numero1, 50_000.0);
        let mut rng = StdRng::seed_from_u64(4);

        let decision = should_renew_contract(&driver, 45.0, &contract, 90.0, &mut rng);

        assert!(decision.should_renew);
        assert!(decision.reason.contains("Leal"));
    }

    #[test]
    fn test_mercenary_wants_more_salary() {
        let driver = sample_driver(30, Some(PrimaryPersonality::Mercenario));
        let contract = sample_contract(TeamRole::Numero1, 100_000.0);
        let mut rng = StdRng::seed_from_u64(5);

        let decision = should_renew_contract(&driver, 75.0, &contract, 90.0, &mut rng);

        assert!(decision.should_renew);
        assert!(decision.new_salary.expect("salary") > 100_000.0);
    }

    fn sample_driver(age: u32, personality: Option<PrimaryPersonality>) -> Driver {
        let mut driver = Driver::new(
            "P001".to_string(),
            "Piloto".to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            age,
            2020,
        );
        driver.personalidade_primaria = personality;
        driver
    }

    fn sample_contract(role: TeamRole, salary: f64) -> Contract {
        Contract::new(
            "C001".to_string(),
            "P001".to_string(),
            "Piloto".to_string(),
            "T001".to_string(),
            "Equipe".to_string(),
            1,
            1,
            salary,
            role,
            "gt4".to_string(),
        )
    }
}
