pub fn derive_financial_state(score: f64) -> &'static str {
    match score {
        value if value >= 75.0 => "elite",
        value if value >= 60.0 => "healthy",
        value if value >= 45.0 => "stable",
        value if value >= 30.0 => "pressured",
        value if value >= 15.0 => "crisis",
        _ => "collapse",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_financial_health_maps_to_elite() {
        assert_eq!(derive_financial_state(90.0), "elite");
    }

    #[test]
    fn low_financial_health_maps_to_collapse() {
        assert_eq!(derive_financial_state(10.0), "collapse");
    }
}
