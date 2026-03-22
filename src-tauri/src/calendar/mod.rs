use std::collections::{HashMap, HashSet};

use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use crate::constants::categories::{
    get_all_categories, get_category_config, has_calendar_conflict, CategoryConfig,
};
use crate::constants::tracks::{
    get_qualifying_duration, get_rain_chance, get_tracks_for_tier, TrackInfo,
};
use crate::models::enums::{RaceStatus, WeatherCondition};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarEntry {
    pub id: String,
    pub season_id: String,
    pub categoria: String,
    pub rodada: i32,
    pub nome: String,
    pub track_id: u32,
    pub track_name: String,
    pub track_config: String,
    pub clima: WeatherCondition,
    pub temperatura: f64,
    pub voltas: i32,
    pub duracao_corrida_min: i32,
    pub duracao_classificacao_min: i32,
    pub status: RaceStatus,
    pub horario: String,
}

const SCHEDULE_HOURS: [&str; 5] = ["10:00", "12:00", "14:00", "16:00", "18:00"];

pub fn generate_calendar_for_category(
    season_id: &str,
    categoria: &str,
    rng: &mut impl Rng,
) -> Result<Vec<CalendarEntry>, String> {
    let mut next_id = 1_u32;
    generate_calendar_for_category_with_constraints(
        season_id,
        categoria,
        &HashMap::new(),
        &mut || {
            let id = format!("R{:03}", next_id);
            next_id += 1;
            id
        },
        rng,
    )
}

pub fn generate_all_calendars(
    season_id: &str,
    rng: &mut impl Rng,
) -> Result<HashMap<String, Vec<CalendarEntry>>, String> {
    let mut next_id = 1_u32;
    generate_all_calendars_with_id_factory(
        season_id,
        &mut || {
            let id = format!("R{:03}", next_id);
            next_id += 1;
            id
        },
        rng,
    )
}

pub(crate) fn generate_all_calendars_with_id_factory<F, R>(
    season_id: &str,
    id_generator: &mut F,
    rng: &mut R,
) -> Result<HashMap<String, Vec<CalendarEntry>>, String>
where
    F: FnMut() -> String,
    R: Rng,
{
    let mut calendars: HashMap<String, Vec<CalendarEntry>> = HashMap::new();

    for category in get_all_categories() {
        let conflicts = calendars
            .iter()
            .filter(|(other_category, _)| {
                has_calendar_conflict(category.id, other_category.as_str())
            })
            .flat_map(|(_, entries)| entries.iter())
            .fold(
                HashMap::<i32, HashSet<u32>>::new(),
                |mut acc: HashMap<i32, HashSet<u32>>, entry| {
                    acc.entry(entry.rodada).or_default().insert(entry.track_id);
                    acc
                },
            );

        let calendar = generate_calendar_for_category_with_constraints(
            season_id,
            category.id,
            &conflicts,
            id_generator,
            rng,
        )?;
        calendars.insert(category.id.to_string(), calendar);
    }

    Ok(calendars)
}

fn generate_calendar_for_category_with_constraints<F, R>(
    season_id: &str,
    categoria: &str,
    banned_tracks_by_round: &HashMap<i32, HashSet<u32>>,
    id_generator: &mut F,
    rng: &mut R,
) -> Result<Vec<CalendarEntry>, String>
where
    F: FnMut() -> String,
    R: Rng,
{
    let config = get_category_config(categoria)
        .ok_or_else(|| format!("Categoria desconhecida: {categoria}"))?;
    let eligible_tracks = get_tracks_for_tier(config.tier);
    if eligible_tracks.len() < config.corridas_por_temporada as usize {
        return Err(format!(
            "Pistas insuficientes para gerar calendario de {}",
            categoria
        ));
    }

    let ordered_tracks = select_tracks(config, &eligible_tracks, banned_tracks_by_round, rng)?;

    let entries = ordered_tracks
        .into_iter()
        .enumerate()
        .map(|(index, track)| {
            build_calendar_entry(
                id_generator(),
                season_id,
                categoria,
                (index + 1) as i32,
                track,
                config,
                rng,
            )
        })
        .collect();

    Ok(entries)
}

fn select_tracks<'a, R: Rng>(
    config: &CategoryConfig,
    eligible_tracks: &'a [&'static TrackInfo],
    banned_tracks_by_round: &HashMap<i32, HashSet<u32>>,
    rng: &mut R,
) -> Result<Vec<&'a TrackInfo>, String> {
    let mut used = HashSet::new();
    let fixed_tracks = select_fixed_tracks(config, eligible_tracks);
    let mut selected = fixed_tracks.clone();
    used.extend(fixed_tracks.iter().map(|track| track.track_id));

    let remaining_needed = config.corridas_por_temporada as usize - selected.len();
    let mut variable_candidates: Vec<&TrackInfo> = eligible_tracks
        .iter()
        .copied()
        .filter(|track| !used.contains(&track.track_id))
        .collect();
    variable_candidates.shuffle(rng);

    for track in variable_candidates.into_iter().take(remaining_needed) {
        used.insert(track.track_id);
        selected.push(track);
    }

    if selected.len() != config.corridas_por_temporada as usize {
        return Err(format!(
            "Nao foi possivel selecionar pistas suficientes para {}",
            config.id
        ));
    }

    if config.tier == 0 {
        selected.shuffle(rng);
    }

    let mut ordered = Vec::with_capacity(selected.len());
    let mut remaining = selected;
    for rodada in 1..=config.corridas_por_temporada as i32 {
        let banned = banned_tracks_by_round.get(&rodada);
        let chosen_index = remaining
            .iter()
            .position(|track| {
                banned
                    .map(|tracks| !tracks.contains(&track.track_id))
                    .unwrap_or(true)
            })
            .or_else(|| {
                eligible_tracks
                    .iter()
                    .copied()
                    .find(|track| {
                        !ordered
                            .iter()
                            .any(|used_track: &&TrackInfo| used_track.track_id == track.track_id)
                            && banned
                                .map(|tracks| !tracks.contains(&track.track_id))
                                .unwrap_or(true)
                    })
                    .map(|replacement| {
                        remaining.push(replacement);
                        remaining.len() - 1
                    })
            });

        let Some(index) = chosen_index else {
            return Err(format!(
                "Nao foi possivel resolver conflito de calendario para {} na rodada {}",
                config.id, rodada
            ));
        };

        ordered.push(remaining.remove(index));
    }

    Ok(ordered)
}

fn select_fixed_tracks<'a>(
    config: &CategoryConfig,
    eligible_tracks: &'a [&'static TrackInfo],
) -> Vec<&'a TrackInfo> {
    let fixed_count = config.pistas_fixas as usize;
    if fixed_count == 0 {
        return Vec::new();
    }

    let start_index = config
        .id
        .bytes()
        .fold(0_usize, |acc, byte| acc + byte as usize)
        % eligible_tracks.len();

    (0..fixed_count)
        .map(|offset| eligible_tracks[(start_index + offset) % eligible_tracks.len()])
        .collect()
}

fn build_calendar_entry<R: Rng>(
    id: String,
    season_id: &str,
    categoria: &str,
    rodada: i32,
    track: &TrackInfo,
    config: &CategoryConfig,
    rng: &mut R,
) -> CalendarEntry {
    let clima = random_weather(track.track_id, rng);
    let temperatura = random_temperature(clima, rng);
    let duracao_corrida_min = resolve_race_duration(config, rng);
    let duracao_classificacao_min = get_qualifying_duration(track.track_id) as i32;
    let voltas = estimate_laps(track, duracao_corrida_min);
    let (track_name, track_config) = split_track_name(track.nome);

    CalendarEntry {
        id,
        season_id: season_id.to_string(),
        categoria: categoria.to_string(),
        rodada,
        nome: format!("Rodada {} - {}", rodada, track.nome_curto),
        track_id: track.track_id,
        track_name,
        track_config,
        clima,
        temperatura,
        voltas,
        duracao_corrida_min,
        duracao_classificacao_min,
        status: RaceStatus::Pendente,
        horario: SCHEDULE_HOURS[rng.gen_range(0..SCHEDULE_HOURS.len())].to_string(),
    }
}

fn random_weather(rain_track_id: u32, rng: &mut impl Rng) -> WeatherCondition {
    let rain_chance = get_rain_chance(rain_track_id);
    if rng.gen::<f64>() >= rain_chance {
        return WeatherCondition::Dry;
    }

    let intensity = rng.gen::<f64>();
    if intensity < 0.40 {
        WeatherCondition::Damp
    } else if intensity < 0.80 {
        WeatherCondition::Wet
    } else {
        WeatherCondition::HeavyRain
    }
}

fn random_temperature(clima: WeatherCondition, rng: &mut impl Rng) -> f64 {
    let (min, max) = match clima {
        WeatherCondition::Dry => (20.0, 35.0),
        WeatherCondition::Damp => (15.0, 25.0),
        WeatherCondition::Wet => (12.0, 22.0),
        WeatherCondition::HeavyRain => (10.0, 20.0),
    };
    (rng.gen_range(min..=max) * 10.0_f64).round() / 10.0_f64
}

fn resolve_race_duration(config: &CategoryConfig, rng: &mut impl Rng) -> i32 {
    if config.duracao_corrida_min > 0 {
        config.duracao_corrida_min as i32
    } else {
        [120, 180, 240, 360][rng.gen_range(0..4)]
    }
}

fn estimate_laps(track: &TrackInfo, duracao_corrida_min: i32) -> i32 {
    let tempo_volta_estimado_min = track.comprimento_km / 2.0;
    ((duracao_corrida_min as f64 / tempo_volta_estimado_min).ceil() as i32).clamp(5, 50)
}

fn split_track_name(full_name: &str) -> (String, String) {
    if let Some((name, config)) = full_name.split_once(" - ") {
        (name.to_string(), config.to_string())
    } else {
        (full_name.to_string(), "Default".to_string())
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::constants::tracks::get_track;

    #[test]
    fn test_generate_calendar_correct_count() {
        let mut rng = StdRng::seed_from_u64(1);
        let gt3 = generate_calendar_for_category("S001", "gt3", &mut rng).expect("gt3 calendar");
        let mazda = generate_calendar_for_category("S001", "mazda_rookie", &mut rng)
            .expect("rookie calendar");

        assert_eq!(gt3.len(), 14);
        assert_eq!(mazda.len(), 5);
    }

    #[test]
    fn test_generate_calendar_no_duplicate_tracks() {
        let mut rng = StdRng::seed_from_u64(2);
        let calendar = generate_calendar_for_category("S001", "gt4", &mut rng).expect("calendar");
        let unique: HashSet<_> = calendar.iter().map(|entry| entry.track_id).collect();
        assert_eq!(unique.len(), calendar.len());
    }

    #[test]
    fn test_generate_calendar_respects_tier_tracks() {
        let mut rng = StdRng::seed_from_u64(3);
        let calendar =
            generate_calendar_for_category("S001", "mazda_rookie", &mut rng).expect("calendar");
        assert!(calendar.iter().all(|entry| {
            get_track(entry.track_id)
                .map(|track| track.gratuita)
                .unwrap_or(false)
        }));
    }

    #[test]
    fn test_generate_calendar_weather_distribution() {
        let mut rng = StdRng::seed_from_u64(4);
        let mut wet_races = 0_usize;
        let mut total_races = 0_usize;

        for _ in 0..100 {
            let calendar =
                generate_calendar_for_category("S001", "gt3", &mut rng).expect("calendar");
            wet_races += calendar
                .iter()
                .filter(|entry| entry.clima != WeatherCondition::Dry)
                .count();
            total_races += calendar.len();
        }

        let ratio = wet_races as f64 / total_races as f64;
        assert!(
            ratio > 0.05 && ratio < 0.35,
            "unexpected wet ratio: {}",
            ratio
        );
    }

    #[test]
    fn test_generate_all_calendars_no_conflicts() {
        let mut rng = StdRng::seed_from_u64(5);
        let calendars = generate_all_calendars("S001", &mut rng).expect("all calendars");

        for (left, right) in [
            ("mazda_rookie", "toyota_rookie"),
            ("mazda_amador", "toyota_amador"),
        ] {
            let left_calendar = calendars.get(left).expect("left calendar");
            let right_calendar = calendars.get(right).expect("right calendar");

            for (left_entry, right_entry) in left_calendar.iter().zip(right_calendar.iter()) {
                assert_ne!(left_entry.track_id, right_entry.track_id);
            }
        }
    }

    #[test]
    fn test_generate_calendar_voltas_reasonable() {
        let mut rng = StdRng::seed_from_u64(6);
        let calendar =
            generate_calendar_for_category("S001", "endurance", &mut rng).expect("calendar");
        assert!(calendar
            .iter()
            .all(|entry| (5..=50).contains(&entry.voltas)));
    }
}
