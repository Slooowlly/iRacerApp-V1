use crate::models::team::Team;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundCashflowSummary {
    pub income: f64,
    pub expenses: f64,
    pub net: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamRoundFinanceContext {
    pub sponsorship_income: f64,
    pub result_bonus: f64,
    pub partial_prize_income: f64,
    pub aid_income: f64,
    pub salary_expense: f64,
    pub event_operations_cost: f64,
    pub structural_maintenance_cost: f64,
    pub technical_investment_cost: f64,
    pub debt_service_cost: f64,
}

pub fn calculate_round_income(
    sponsorship_income: f64,
    result_bonus: f64,
    partial_prize_income: f64,
    aid_income: f64,
) -> f64 {
    sponsorship_income.max(0.0)
        + result_bonus.max(0.0)
        + partial_prize_income.max(0.0)
        + aid_income.max(0.0)
}

pub fn calculate_round_expenses(
    salary_expense: f64,
    event_operations_cost: f64,
    structural_maintenance_cost: f64,
    technical_investment_cost: f64,
    debt_service_cost: f64,
) -> f64 {
    salary_expense.max(0.0)
        + event_operations_cost.max(0.0)
        + structural_maintenance_cost.max(0.0)
        + technical_investment_cost.max(0.0)
        + debt_service_cost.max(0.0)
}

pub fn summarize_round_cashflow(income: f64, expenses: f64) -> RoundCashflowSummary {
    RoundCashflowSummary {
        income,
        expenses,
        net: income - expenses,
    }
}

pub fn apply_round_cashflow(
    team: &mut Team,
    context: TeamRoundFinanceContext,
) -> RoundCashflowSummary {
    let income = calculate_round_income(
        context.sponsorship_income,
        context.result_bonus,
        context.partial_prize_income,
        context.aid_income,
    );
    let expenses = calculate_round_expenses(
        context.salary_expense,
        context.event_operations_cost,
        context.structural_maintenance_cost,
        context.technical_investment_cost,
        context.debt_service_cost,
    );
    let summary = summarize_round_cashflow(income, expenses);

    team.last_round_income = summary.income;
    team.last_round_expenses = summary.expenses;
    team.last_round_net = summary.net;
    team.cash_balance += summary.net;

    if team.cash_balance < -100_000.0 {
        let financed_amount = -100_000.0 - team.cash_balance;
        team.debt_balance += financed_amount;
        team.cash_balance = -100_000.0;
    }

    if team.parachute_payment_remaining > 0.0 {
        team.parachute_payment_remaining =
            (team.parachute_payment_remaining - context.aid_income.max(0.0)).max(0.0);
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::team::placeholder_team_from_db;

    #[test]
    fn round_income_stays_positive_for_basic_team_revenue() {
        let round_income = calculate_round_income(125_000.0, 25_000.0, 8_000.0, 0.0);
        assert!(round_income > 0.0);
    }

    #[test]
    fn round_expenses_stay_positive_for_basic_team_costs() {
        let round_expenses = calculate_round_expenses(60_000.0, 22_000.0, 15_000.0, 9_500.0, 3_000.0);
        assert!(round_expenses > 0.0);
    }

    #[test]
    fn round_cashflow_summary_tracks_net_value() {
        let summary = summarize_round_cashflow(158_000.0, 121_500.0);

        assert_eq!(summary.net, 36_500.0);
    }

    #[test]
    fn apply_round_cashflow_updates_team_snapshot() {
        let mut team = placeholder_team_from_db(
            "T001".to_string(),
            "Equipe Financeira".to_string(),
            "gt3".to_string(),
            "2026-01-01".to_string(),
        );
        team.cash_balance = 500_000.0;

        let summary = apply_round_cashflow(
            &mut team,
            TeamRoundFinanceContext {
                sponsorship_income: 120_000.0,
                result_bonus: 25_000.0,
                partial_prize_income: 10_000.0,
                aid_income: 0.0,
                salary_expense: 45_000.0,
                event_operations_cost: 20_000.0,
                structural_maintenance_cost: 15_000.0,
                technical_investment_cost: 18_000.0,
                debt_service_cost: 2_500.0,
            },
        );

        assert_eq!(team.last_round_income, summary.income);
        assert_eq!(team.last_round_expenses, summary.expenses);
        assert_eq!(team.last_round_net, summary.net);
        assert_eq!(team.cash_balance, 554_500.0);
    }
}
