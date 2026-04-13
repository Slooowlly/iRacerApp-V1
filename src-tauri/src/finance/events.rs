pub fn debt_service(debt_balance: f64, round_interest_rate: f64) -> f64 {
    if debt_balance <= 0.0 {
        return 0.0;
    }

    debt_balance * round_interest_rate.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debt_service_is_positive_when_team_owes_money() {
        assert!(debt_service(100_000.0, 0.015) > 0.0);
    }
}
