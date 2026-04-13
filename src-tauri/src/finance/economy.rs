#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalEconomicHealth {
    Boom,
    Neutral,
    Recession,
}

pub fn economy_income_modifier(health: GlobalEconomicHealth) -> f64 {
    match health {
        GlobalEconomicHealth::Boom => 1.12,
        GlobalEconomicHealth::Neutral => 1.0,
        GlobalEconomicHealth::Recession => 0.9,
    }
}

pub fn economy_cost_modifier(health: GlobalEconomicHealth) -> f64 {
    match health {
        GlobalEconomicHealth::Boom => 1.08,
        GlobalEconomicHealth::Neutral => 1.0,
        GlobalEconomicHealth::Recession => 0.92,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_economy_modifier_defaults_to_one() {
        assert_eq!(economy_income_modifier(GlobalEconomicHealth::Neutral), 1.0);
    }

    #[test]
    fn recession_reduces_costs_as_well() {
        assert!(economy_cost_modifier(GlobalEconomicHealth::Recession) < 1.0);
    }
}
