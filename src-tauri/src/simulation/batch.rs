use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SimultaneousResults {
    pub categories_simulated: Vec<CategorySimResult>,
    pub total_races_simulated: i32,
    pub highlights: Vec<SimHighlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategorySimResult {
    pub category_id: String,
    pub category_name: String,
    pub races_simulated: i32,
    pub results: Vec<BriefRaceResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BriefRaceResult {
    pub race_id: String,
    pub track_name: String,
    pub winner_name: String,
    pub winner_team: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimHighlight {
    pub headline: String,
    pub category: String,
}

pub fn races_should_be_completed(
    player_race_number: i32,
    player_total_races: i32,
    other_total_races: i32,
) -> i32 {
    if player_total_races <= 0 || other_total_races <= 0 {
        return 0;
    }

    if player_race_number >= player_total_races {
        return other_total_races;
    }

    let proportion = player_race_number as f64 / player_total_races as f64;
    (proportion * other_total_races as f64).round() as i32
}

#[cfg(test)]
mod tests {
    use super::races_should_be_completed;

    #[test]
    fn test_races_should_be_completed_proportion() {
        assert_eq!(races_should_be_completed(3, 5, 14), 8);
    }

    #[test]
    fn test_races_should_be_completed_last_race() {
        assert_eq!(races_should_be_completed(5, 5, 14), 14);
    }
}
