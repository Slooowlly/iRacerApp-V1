use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};
use tauri::{AppHandle, Manager};

use crate::commands::career::{
    get_calendar_for_category_in_base_dir, get_drivers_by_category_in_base_dir,
    get_news_in_base_dir, get_teams_standings_in_base_dir, load_career_in_base_dir,
};
use crate::commands::career_types::{
    NewsTabBootstrap, NewsTabFilterOption, NewsTabHero, NewsTabScopeMeta, NewsTabScopeTab,
    NewsTabSnapshot, NewsTabSnapshotRequest, NewsTabStory, NewsTabStoryBlock,
};
use crate::commands::news_editorial::{
    build_editorial_blocks, build_story_deck, classify_editorial_story_type,
    editorial_block_labels, editorial_slot_indexes, EditorialExtras,
};
use crate::commands::news_helpers::{
    build_meta_label, build_story_time_label, format_display_date_label, freshness_bonus,
    importance_label, importance_rank, scope_class_label, story_accent, story_sort_key,
    team_color_pair, team_presence_label,
};
use crate::config::app_config::AppConfig;
use crate::constants::categories;
use crate::db::connection::Database;
use crate::db::queries::contracts as contract_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::rivalries as rivalry_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::teams as team_queries;
use crate::models::rivalry::Rivalry;
use crate::news::{NewsImportance, NewsItem, NewsType};
use crate::public_presence::team::derive_team_public_presence;

const PRIMARY_FILTER_IDS: [&str; 4] = ["Corridas", "Pilotos", "Equipes", "Mercado"];
const FAMOUS_FILTER_IDS: [&str; 3] = ["Pilotos", "Equipes", "Mercado"];

#[tauri::command]
pub async fn get_news_tab_bootstrap(
    app: AppHandle,
    career_id: String,
) -> Result<NewsTabBootstrap, String> {
    let base_dir = app_data_dir(&app)?;
    get_news_tab_bootstrap_in_base_dir(&base_dir, &career_id)
}

#[tauri::command]
pub async fn get_news_tab_snapshot(
    app: AppHandle,
    career_id: String,
    request: NewsTabSnapshotRequest,
) -> Result<NewsTabSnapshot, String> {
    let base_dir = app_data_dir(&app)?;
    get_news_tab_snapshot_in_base_dir(&base_dir, &career_id, request)
}

pub(crate) fn get_news_tab_bootstrap_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<NewsTabBootstrap, String> {
    let career = load_career_in_base_dir(base_dir, career_id)?;
    let current_round = career.season.rodada_atual;
    let calendar = get_calendar_for_category_in_base_dir(
        base_dir,
        career_id,
        &career.player_team.categoria,
    )
    .unwrap_or_default();
    let last_race = calendar.iter().filter(|r| r.rodada < current_round).max_by_key(|r| r.rodada);
    let next_race = calendar.iter().find(|r| r.rodada >= current_round);
    let pub_date_label = last_race
        .map(|r| format_display_date(&r.display_date))
        .unwrap_or_else(|| career.season.ano.to_string());
    let last_race_name = last_race.map(|r| r.track_name.clone());
    let next_race_date_label = next_race.map(|r| format_display_date(&r.display_date));
    let next_race_name = next_race.map(|r| r.track_name.clone());
    Ok(NewsTabBootstrap {
        default_scope_type: "category".to_string(),
        default_scope_id: career.player_team.categoria.clone(),
        default_primary_filter: None,
        scopes: build_scope_tabs(),
        season_number: career.season.numero,
        season_year: career.season.ano,
        current_round,
        total_rounds: career.season.total_rodadas,
        pub_date_label,
        last_race_name,
        next_race_date_label,
        next_race_name,
    })
}

fn format_display_date(date_str: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| format!("{} {} de {}", d.day(), MONTHS[d.month0() as usize], d.year()))
        .unwrap_or_else(|_| date_str.to_string())
}

pub(crate) fn get_news_tab_snapshot_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    request: NewsTabSnapshotRequest,
) -> Result<NewsTabSnapshot, String> {
    let context = load_context(base_dir, career_id)?;
    let scope_type = normalize_scope_type(&request.scope_type);
    let scope_id = normalize_scope_id(
        scope_type,
        &request.scope_id,
        &context.career.player_team.categoria,
    );
    let scope_class = normalize_scope_class(&scope_id, request.scope_class.as_deref());
    let primary_filter = normalize_primary_filter(scope_type, request.primary_filter.as_deref());
    let primary_filters = build_primary_filters(scope_type);
    let scope_label = if scope_type == "famous" {
        "Mais famosos".to_string()
    } else {
        build_scope_label(&context, &scope_id, scope_class.as_deref())
    };
    let scoped_items = select_scope_items(&context, scope_type, &scope_id, scope_class.as_deref())?;
    let primary_items = select_primary_items(
        &context,
        scope_type,
        &scope_id,
        scope_class.as_deref(),
        primary_filter.as_deref(),
        scoped_items,
    );
    let contextual_filters = if scope_type == "famous" {
        build_famous_context_filters(
            &context,
            primary_filter.as_deref(),
            &primary_items,
            request.context_type.as_deref(),
            request.context_id.as_deref(),
        )?
    } else {
        build_category_context_filters(
            &context,
            &scope_id,
            scope_class.as_deref(),
            primary_filter.as_deref(),
            &primary_items,
            request.context_type.as_deref(),
            request.context_id.as_deref(),
        )?
    };
    let selection = resolve_context_selection(
        &contextual_filters,
        request.context_type.as_deref(),
        request.context_id.as_deref(),
    );
    let mut selected_items = primary_items;
    if let Some(selection) = selection.as_ref() {
        selected_items.retain(|item| story_matches_context(&context, item, selection));
    }
    let stories = build_stories(&context, selected_items);

    Ok(NewsTabSnapshot {
        hero: build_hero(
            &context,
            scope_type,
            &scope_id,
            &scope_label,
            scope_class.as_deref(),
            primary_filter.as_deref(),
        ),
        primary_filters,
        contextual_filters,
        stories,
        scope_meta: NewsTabScopeMeta {
            scope_type: scope_type.to_string(),
            scope_id,
            scope_label,
            scope_class,
            primary_filter,
            context_type: selection.as_ref().map(|value| value.kind.clone()),
            context_id: selection.as_ref().map(|value| value.id.clone()),
            context_label: selection.as_ref().map(|value| value.label.clone()),
            is_special: scope_type == "famous",
        },
    })
}

pub(crate) struct NewsTabContext {
    pub(crate) base_dir: PathBuf,
    pub(crate) career_id: String,
    pub(crate) db: Database,
    pub(crate) career: crate::commands::career_types::CareerData,
    pub(crate) all_news: Vec<NewsItem>,
    pub(crate) newest_timestamp: i64,
    pub(crate) driver_names: HashMap<String, String>,
    pub(crate) driver_media: HashMap<String, f64>,
    pub(crate) team_names: HashMap<String, String>,
    pub(crate) team_colors: HashMap<String, (String, String)>,
    pub(crate) team_driver_ids: HashMap<String, Vec<String>>,
    pub(crate) category_names: HashMap<String, String>,
    pub(crate) race_rounds: HashMap<String, i32>,
    pub(crate) race_labels: HashMap<String, String>,
    pub(crate) race_dates: HashMap<String, String>,
    pub(crate) max_preseason_week_by_season: HashMap<i32, i32>,
    /// "category_id:driver_id" → posição no campeonato
    pub(crate) driver_positions: HashMap<String, i32>,
    /// "category_id:driver_id" → pontos arredondados
    pub(crate) driver_points: HashMap<String, i32>,
    /// team_id → posição no campeonato da categoria da equipe
    pub(crate) team_positions: HashMap<String, i32>,
    /// team_id → pontos da equipe
    pub(crate) team_points: HashMap<String, i32>,
    /// category_id → próxima corrida da categoria
    pub(crate) next_race_by_category: HashMap<String, NextRaceInfo>,
    /// team_id → tier de presença pública ("elite", "alta", "relevante", "baixa")
    pub(crate) team_public_presence: HashMap<String, String>,
}

pub(crate) struct NextRaceInfo {
    pub(crate) label: String,
    pub(crate) date_label: String,
    pub(crate) round: i32,
}

#[derive(Clone)]
struct ContextSelection {
    kind: String,
    id: String,
    label: String,
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))
}

fn load_context(base_dir: &Path, career_id: &str) -> Result<NewsTabContext, String> {
    let career = load_career_in_base_dir(base_dir, career_id)?;
    let config = AppConfig::load_or_default(base_dir);
    let db_path = config.saves_dir().join(career_id).join("career.db");
    let db = Database::open_existing(&db_path)
        .map_err(|e| format!("Falha ao abrir banco da carreira: {e}"))?;
    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let all_news = get_news_in_base_dir(
        base_dir,
        career_id,
        Some(active_season.numero),
        None,
        Some(400),
    )?;
    let newest_timestamp = all_news.first().map(|item| item.timestamp).unwrap_or(0);

    let all_drivers = driver_queries::get_all_drivers(&db.conn)
        .map_err(|e| format!("Falha ao buscar pilotos do mundo: {e}"))?;
    let all_teams = team_queries::get_all_teams(&db.conn)
        .map_err(|e| format!("Falha ao buscar equipes do mundo: {e}"))?;

    let current_round = career.season.rodada_atual;
    let mut race_rounds = HashMap::new();
    let mut race_labels = HashMap::new();
    let mut race_dates = HashMap::new();
    let mut next_race_by_category: HashMap<String, NextRaceInfo> = HashMap::new();
    for category in categories::get_all_categories() {
        for race in get_calendar_for_category_in_base_dir(base_dir, career_id, category.id)? {
            race_rounds.insert(race.id.clone(), race.rodada);
            race_labels.insert(
                format!("{}:{}", category.id, race.rodada),
                race.track_name.clone(),
            );
            race_dates.insert(
                format!("{}:{}", category.id, race.rodada),
                race.display_date.clone(),
            );
            if race.rodada >= current_round
                && !next_race_by_category.contains_key(category.id)
            {
                let date_label = format_display_date_label(&race.display_date)
                    .unwrap_or_default();
                next_race_by_category.insert(
                    category.id.to_string(),
                    NextRaceInfo {
                        label: race.track_name,
                        date_label,
                        round: race.rodada,
                    },
                );
            }
        }
    }
    // driver standings: chave composta "category_id:driver_id"
    let mut driver_groups: HashMap<String, Vec<&crate::models::driver::Driver>> = HashMap::new();
    for driver in &all_drivers {
        if let Some(cat) = driver.categoria_atual.as_deref() {
            driver_groups.entry(cat.to_string()).or_default().push(driver);
        }
    }
    let mut driver_positions: HashMap<String, i32> = HashMap::new();
    let mut driver_points: HashMap<String, i32> = HashMap::new();
    for (cat_id, group) in &mut driver_groups {
        group.sort_by(|a, b| {
            b.stats_temporada.pontos
                .partial_cmp(&a.stats_temporada.pontos)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.stats_temporada.vitorias.cmp(&a.stats_temporada.vitorias))
                .then(b.stats_temporada.podios.cmp(&a.stats_temporada.podios))
        });
        for (pos, driver) in group.iter().enumerate() {
            let key = format!("{cat_id}:{}", driver.id);
            driver_positions.insert(key.clone(), pos as i32 + 1);
            driver_points.insert(key, driver.stats_temporada.pontos.round() as i32);
        }
    }

    // team standings: chave team_id (uma categoria por equipe)
    let mut team_groups: HashMap<String, Vec<&crate::models::team::Team>> = HashMap::new();
    for team in &all_teams {
        team_groups.entry(team.categoria.clone()).or_default().push(team);
    }
    let mut team_positions: HashMap<String, i32> = HashMap::new();
    let mut team_points: HashMap<String, i32> = HashMap::new();
    for group in team_groups.values_mut() {
        group.sort_by(|a, b| {
            b.stats_pontos
                .cmp(&a.stats_pontos)
                .then(b.stats_vitorias.cmp(&a.stats_vitorias))
                .then(b.stats_podios.cmp(&a.stats_podios))
        });
        for (pos, team) in group.iter().enumerate() {
            team_positions.insert(team.id.clone(), pos as i32 + 1);
            team_points.insert(team.id.clone(), team.stats_pontos);
        }
    }

    // presença pública por equipe
    let team_public_presence: HashMap<String, String> = all_teams
        .iter()
        .map(|team| {
            let driver_media_values: Vec<f64> = [&team.piloto_1_id, &team.piloto_2_id]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .filter_map(|id| {
                    all_drivers
                        .iter()
                        .find(|d| &d.id == id)
                        .map(|d| d.atributos.midia)
                })
                .collect();
            let tier =
                team_presence_label(&derive_team_public_presence(&driver_media_values).tier)
                    .to_string();
            (team.id.clone(), tier)
        })
        .collect();

    let mut max_preseason_week_by_season = HashMap::new();
    for item in &all_news {
        if let Some(week) = item.semana_pretemporada.filter(|week| *week > 0) {
            max_preseason_week_by_season
                .entry(item.temporada)
                .and_modify(|current: &mut i32| *current = (*current).max(week))
                .or_insert(week);
        }
    }

    Ok(NewsTabContext {
        base_dir: base_dir.to_path_buf(),
        career_id: career_id.to_string(),
        db,
        career,
        all_news,
        newest_timestamp,
        driver_names: all_drivers
            .iter()
            .map(|d| (d.id.clone(), d.nome.clone()))
            .collect(),
        driver_media: all_drivers
            .iter()
            .map(|d| (d.id.clone(), d.atributos.midia))
            .collect(),
        team_names: all_teams
            .iter()
            .map(|t| (t.id.clone(), t.nome.clone()))
            .collect(),
        team_colors: all_teams
            .iter()
            .map(|team| {
                (
                    team.id.clone(),
                    (team.cor_primaria.clone(), team.cor_secundaria.clone()),
                )
            })
            .collect(),
        team_driver_ids: all_teams
            .iter()
            .map(|team| {
                let mut ids = Vec::new();
                if let Some(id) = team.piloto_1_id.clone() {
                    ids.push(id);
                }
                if let Some(id) = team.piloto_2_id.clone() {
                    ids.push(id);
                }
                (team.id.clone(), ids)
            })
            .collect(),
        category_names: categories::get_all_categories()
            .iter()
            .map(|category| (category.id.to_string(), category.nome.to_string()))
            .collect(),
        race_rounds,
        race_labels,
        race_dates,
        max_preseason_week_by_season,
        driver_positions,
        driver_points,
        team_positions,
        team_points,
        next_race_by_category,
        team_public_presence,
    })
}

fn build_scope_tabs() -> Vec<NewsTabScopeTab> {
    let mut tabs: Vec<NewsTabScopeTab> = categories::get_all_categories()
        .iter()
        .map(|category| NewsTabScopeTab {
            id: category.id.to_string(),
            label: category.nome.to_string(),
            short_label: category.nome_curto.to_string(),
            scope_type: "category".to_string(),
            special: false,
        })
        .collect();
    tabs.push(NewsTabScopeTab {
        id: "mais_famosos".to_string(),
        label: "Mais famosos".to_string(),
        short_label: "Mais famosos".to_string(),
        scope_type: "famous".to_string(),
        special: true,
    });
    tabs
}

fn build_primary_filters(scope_type: &str) -> Vec<NewsTabFilterOption> {
    let ids = if scope_type == "famous" {
        &FAMOUS_FILTER_IDS[..]
    } else {
        &PRIMARY_FILTER_IDS[..]
    };
    ids.iter()
        .map(|id| NewsTabFilterOption {
            id: (*id).to_string(),
            label: (*id).to_string(),
            meta: None,
            tone: Some(if *id == "Mercado" {
                "warm".to_string()
            } else {
                "cool".to_string()
            }),
            kind: Some("tag".to_string()),
            color_primary: None,
            color_secondary: None,
        })
        .collect()
}

fn build_scope_label(context: &NewsTabContext, scope_id: &str, scope_class: Option<&str>) -> String {
    let base_label = context
        .category_names
        .get(scope_id)
        .cloned()
        .unwrap_or_else(|| scope_id.to_string());

    match scope_class {
        Some(class_name) => format!("{base_label} · {}", scope_class_label(class_name)),
        None => base_label,
    }
}

fn scope_team_ids(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
) -> Result<HashSet<String>, String> {
    if let Some(scope_class) = scope_class {
        return Ok(team_queries::get_teams_by_category_and_class(
            &context.db.conn,
            category_id,
            scope_class,
        )
        .map_err(|e| format!("Falha ao buscar equipes da classe {scope_class}: {e}"))?
        .into_iter()
        .map(|team| team.id)
        .collect());
    }

    Ok(get_teams_standings_in_base_dir(
        &context.base_dir,
        &context.career_id,
        category_id,
    )?
    .into_iter()
    .map(|team| team.id)
    .collect())
}

fn scope_driver_ids(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
    team_ids: &HashSet<String>,
) -> Result<HashSet<String>, String> {
    if scope_class.is_none() {
        return Ok(get_drivers_by_category_in_base_dir(
            &context.base_dir,
            &context.career_id,
            category_id,
        )?
        .into_iter()
        .map(|driver| driver.id)
        .collect());
    }

    let mut driver_ids = HashSet::new();
    for team_id in team_ids {
        if let Some(ids) = context.team_driver_ids.get(team_id) {
            driver_ids.extend(ids.iter().cloned());
        }

        for contract in contract_queries::get_active_contracts_for_team(&context.db.conn, team_id)
            .map_err(|e| format!("Falha ao buscar lineup ativo da equipe {team_id}: {e}"))?
        {
            driver_ids.insert(contract.piloto_id);
        }
    }

    Ok(driver_ids)
}

fn scope_drivers(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
) -> Result<Vec<crate::commands::career_types::DriverSummary>, String> {
    let mut drivers =
        get_drivers_by_category_in_base_dir(&context.base_dir, &context.career_id, category_id)?;
    if let Some(scope_class) = scope_class {
        let team_ids = scope_team_ids(context, category_id, Some(scope_class))?;
        let driver_ids = scope_driver_ids(context, category_id, Some(scope_class), &team_ids)?;
        drivers.retain(|driver| driver_ids.contains(&driver.id));
    }
    Ok(drivers)
}

fn scope_team_standings(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
) -> Result<Vec<crate::commands::career_types::TeamStanding>, String> {
    let mut teams =
        get_teams_standings_in_base_dir(&context.base_dir, &context.career_id, category_id)?;
    if let Some(scope_class) = scope_class {
        let team_ids = scope_team_ids(context, category_id, Some(scope_class))?;
        teams.retain(|team| team_ids.contains(&team.id));
    }
    Ok(teams)
}

fn build_category_context_filters(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
    primary_filter: Option<&str>,
    scoped_items: &[NewsItem],
    requested_context_type: Option<&str>,
    requested_context_id: Option<&str>,
) -> Result<Vec<NewsTabFilterOption>, String> {
    match primary_filter {
        Some("Corridas") => Ok(get_calendar_for_category_in_base_dir(
            &context.base_dir,
            &context.career_id,
            category_id,
        )?
        .into_iter()
        .map(|race| NewsTabFilterOption {
            id: race.id,
            label: race.track_name,
            meta: Some(format!("R{}", race.rodada)),
            tone: Some("cool".to_string()),
            kind: Some("race".to_string()),
            color_primary: None,
            color_secondary: None,
        })
        .collect()),
        Some("Pilotos") => build_pilot_context_filters(
            context,
            category_id,
            scope_class,
            requested_context_type,
            requested_context_id,
        ),
        Some("Equipes") => Ok(scope_team_standings(context, category_id, scope_class)?
        .into_iter()
        .map(|team| {
            let (color_primary, color_secondary) = team_color_pair(context, &team.id);
            NewsTabFilterOption {
                id: team.id,
                label: team.nome,
                meta: Some(format!("{} pts", team.pontos)),
                tone: Some("cool".to_string()),
                kind: Some("team".to_string()),
                color_primary,
                color_secondary,
            }
        })
        .collect()),
        Some("Mercado") => Ok(build_entity_context_filters(context, scoped_items, 6)),
        _ => Ok(Vec::new()),
    }
}

fn build_famous_context_filters(
    context: &NewsTabContext,
    primary_filter: Option<&str>,
    scoped_items: &[NewsItem],
    _requested_context_type: Option<&str>,
    _requested_context_id: Option<&str>,
) -> Result<Vec<NewsTabFilterOption>, String> {
    match primary_filter {
        Some("Pilotos") => Ok(build_famous_driver_filters(context, 8)),
        Some("Equipes") => Ok(build_famous_team_filters(context, 8)),
        Some("Mercado") => Ok(build_entity_context_filters(context, scoped_items, 6)),
        _ => Ok(Vec::new()),
    }
}

fn build_pilot_context_filters(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
    requested_context_type: Option<&str>,
    requested_context_id: Option<&str>,
) -> Result<Vec<NewsTabFilterOption>, String> {
    let drivers = scope_drivers(context, category_id, scope_class)?;
    let driver_map: HashMap<String, String> = drivers
        .iter()
        .map(|driver| (driver.id.clone(), driver.nome.clone()))
        .collect();
    let active_driver_id = match (requested_context_type, requested_context_id) {
        (Some("driver"), Some(driver_id)) if driver_map.contains_key(driver_id) => {
            Some(driver_id.to_string())
        }
        (Some("rivalry"), Some(rivalry_id)) => rivalry_id
            .split('|')
            .next()
            .filter(|driver_id| driver_map.contains_key(*driver_id))
            .map(|value| value.to_string()),
        _ => None,
    };

    if let Some(active_driver_id) = active_driver_id {
        let mut filters = vec![NewsTabFilterOption {
            id: active_driver_id.clone(),
            label: driver_map
                .get(&active_driver_id)
                .cloned()
                .unwrap_or_else(|| active_driver_id.clone()),
            meta: Some("Piloto em foco".to_string()),
            tone: Some("accent".to_string()),
            kind: Some("driver".to_string()),
            color_primary: None,
            color_secondary: None,
        }];

        for rivalry in build_rivalries_for_category(context, category_id, scope_class)? {
            let other_driver_id = if rivalry.piloto1_id == active_driver_id {
                Some(rivalry.piloto2_id.clone())
            } else if rivalry.piloto2_id == active_driver_id {
                Some(rivalry.piloto1_id.clone())
            } else {
                None
            };
            let Some(other_driver_id) = other_driver_id else {
                continue;
            };
            let active_driver_label = driver_map
                .get(&active_driver_id)
                .cloned()
                .unwrap_or_else(|| active_driver_id.clone());
            let other_driver_label = driver_map
                .get(&other_driver_id)
                .cloned()
                .unwrap_or_else(|| other_driver_id.clone());
            filters.push(NewsTabFilterOption {
                id: format!("{}|{}", active_driver_id, other_driver_id),
                label: format!("{active_driver_label} vs {other_driver_label}"),
                meta: Some(format!("{:.0}% intensidade", rivalry.perceived_intensity())),
                tone: Some("warm".to_string()),
                kind: Some("rivalry".to_string()),
                color_primary: None,
                color_secondary: None,
            });
        }

        return Ok(filters);
    }

    Ok(drivers
        .into_iter()
        .map(|driver| NewsTabFilterOption {
            id: driver.id,
            label: driver.nome,
            meta: Some(format!("{} pts", driver.pontos)),
            tone: Some(if driver.is_jogador {
                "accent".to_string()
            } else {
                "cool".to_string()
            }),
            kind: Some("driver".to_string()),
            color_primary: None,
            color_secondary: None,
        })
        .collect())
}

fn build_rivalries_for_category(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
) -> Result<Vec<Rivalry>, String> {
    let team_ids = scope_team_ids(context, category_id, scope_class)?;
    let driver_ids = scope_driver_ids(context, category_id, scope_class, &team_ids)?;
    let mut rivalries: Vec<Rivalry> = rivalry_queries::get_all_rivalries(&context.db.conn)
        .map_err(|e| format!("Falha ao buscar rivalidades: {e}"))?
        .into_iter()
        .filter(|r| driver_ids.contains(&r.piloto1_id) && driver_ids.contains(&r.piloto2_id))
        .collect();
    rivalries.sort_by(|left, right| {
        right
            .perceived_intensity()
            .partial_cmp(&left.perceived_intensity())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(rivalries)
}

fn build_famous_driver_filters(context: &NewsTabContext, limit: usize) -> Vec<NewsTabFilterOption> {
    let mut items: Vec<(String, String, f64)> = context
        .driver_media
        .iter()
        .filter_map(|(id, media)| {
            context
                .driver_names
                .get(id)
                .map(|name| (id.clone(), name.clone(), *media))
        })
        .collect();
    items.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
        .into_iter()
        .take(limit)
        .map(|(id, label, media)| NewsTabFilterOption {
            id,
            label,
            meta: Some(format!("midia {:.0}", media)),
            tone: Some("accent".to_string()),
            kind: Some("driver".to_string()),
            color_primary: None,
            color_secondary: None,
        })
        .collect()
}

fn build_famous_team_filters(context: &NewsTabContext, limit: usize) -> Vec<NewsTabFilterOption> {
    let mut scored: Vec<(String, String, f64, String)> = context
        .team_driver_ids
        .iter()
        .filter_map(|(team_id, driver_ids)| {
            let label = context.team_names.get(team_id)?.clone();
            let medias: Vec<f64> = driver_ids
                .iter()
                .filter_map(|id| context.driver_media.get(id).copied())
                .collect();
            let presence = derive_team_public_presence(&medias);
            Some((
                team_id.clone(),
                label,
                presence.raw_score,
                team_presence_label(&presence.tier).to_string(),
            ))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(id, label, raw, tier)| {
            let (color_primary, color_secondary) = team_color_pair(context, &id);
            NewsTabFilterOption {
                id,
                label,
                meta: Some(format!("{tier} {:.0}", raw)),
                tone: Some("accent".to_string()),
                kind: Some("team".to_string()),
                color_primary,
                color_secondary,
            }
        })
        .collect()
}

fn build_entity_context_filters(
    context: &NewsTabContext,
    items: &[NewsItem],
    limit: usize,
) -> Vec<NewsTabFilterOption> {
    let mut scored: HashMap<(String, String), f64> = HashMap::new();
    for item in items {
        let weight = importance_rank(&item.importancia) as f64 + 1.0;
        if let Some(team_id) = item.team_id.as_ref() {
            *scored
                .entry(("team".to_string(), team_id.clone()))
                .or_insert(0.0) += weight;
        } else if let Some(driver_id) = item.driver_id.as_ref() {
            *scored
                .entry(("driver".to_string(), driver_id.clone()))
                .or_insert(0.0) += weight;
        }
    }

    let mut ranked = scored.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked
        .into_iter()
        .take(limit)
        .filter_map(|((kind, id), score)| {
            if kind == "team" {
                context.team_names.get(&id).map(|label| {
                    let (color_primary, color_secondary) = team_color_pair(context, &id);
                    NewsTabFilterOption {
                        id,
                        label: label.clone(),
                        meta: Some(format!("Equipe em pauta {:.0}", score)),
                        tone: Some("cool".to_string()),
                        kind: Some("team".to_string()),
                        color_primary,
                        color_secondary,
                    }
                })
            } else {
                context
                    .driver_names
                    .get(&id)
                    .map(|label| NewsTabFilterOption {
                        id,
                        label: label.clone(),
                        meta: Some(format!("Piloto em pauta {:.0}", score)),
                        tone: Some("accent".to_string()),
                        kind: Some("driver".to_string()),
                        color_primary: None,
                        color_secondary: None,
                    })
            }
        })
        .collect()
}

fn resolve_context_selection(
    contextual_filters: &[NewsTabFilterOption],
    context_type: Option<&str>,
    context_id: Option<&str>,
) -> Option<ContextSelection> {
    let kind = context_type?;
    let id = context_id?;
    let matched = contextual_filters
        .iter()
        .find(|filter| filter.id == id && filter.kind.as_deref() == Some(kind))?;
    Some(ContextSelection {
        kind: matched.kind.clone().unwrap_or_default(),
        id: matched.id.clone(),
        label: matched.label.clone(),
    })
}

fn select_scope_items(
    context: &NewsTabContext,
    scope_type: &str,
    scope_id: &str,
    scope_class: Option<&str>,
) -> Result<Vec<NewsItem>, String> {
    let driver_ids_in_scope: HashSet<String> = if scope_type == "category" {
        let team_ids = scope_team_ids(context, scope_id, scope_class)?;
        scope_driver_ids(context, scope_id, scope_class, &team_ids)?
    } else {
        HashSet::new()
    };
    let team_ids_in_scope: HashSet<String> = if scope_type == "category" {
        scope_team_ids(context, scope_id, scope_class)?
    } else {
        HashSet::new()
    };
    let famous_driver_ids: HashSet<String> = build_famous_driver_filters(context, 8)
        .into_iter()
        .map(|filter| filter.id)
        .collect();
    let famous_team_ids: HashSet<String> = build_famous_team_filters(context, 8)
        .into_iter()
        .map(|filter| filter.id)
        .collect();

    Ok(context
        .all_news
        .iter()
        .filter(|item| {
            if scope_type == "famous" {
                story_is_famous(item, &famous_driver_ids, &famous_team_ids)
            } else {
                story_belongs_to_category(
                    item,
                    scope_id,
                    scope_class,
                    &driver_ids_in_scope,
                    &team_ids_in_scope,
                )
            }
        })
        .cloned()
        .collect())
}

fn select_primary_items(
    context: &NewsTabContext,
    scope_type: &str,
    scope_id: &str,
    scope_class: Option<&str>,
    primary_filter: Option<&str>,
    items: Vec<NewsItem>,
) -> Vec<NewsItem> {
    if primary_filter.is_none() {
        if scope_type == "famous" {
            return select_public_briefing_items(context, items);
        }
        return select_category_briefing_items(context, scope_id, scope_class, items);
    }

    filter_items_for_primary(context, scope_id, primary_filter, items)
}

fn filter_items_for_primary(
    context: &NewsTabContext,
    scope_id: &str,
    primary_filter: Option<&str>,
    items: Vec<NewsItem>,
) -> Vec<NewsItem> {
    let mut filtered = items
        .into_iter()
        .filter(|item| story_matches_primary_filter(context, scope_id, item, primary_filter))
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| story_sort_key(right).cmp(&story_sort_key(left)));
    filtered
}

fn select_category_briefing_items(
    context: &NewsTabContext,
    category_id: &str,
    scope_class: Option<&str>,
    items: Vec<NewsItem>,
) -> Vec<NewsItem> {
    let class_ids: Option<(HashSet<String>, HashSet<String>)> = if scope_class.is_some() {
        scope_team_ids(context, category_id, scope_class)
            .ok()
            .and_then(|team_ids| {
                scope_driver_ids(context, category_id, scope_class, &team_ids)
                    .ok()
                    .map(|driver_ids| (team_ids, driver_ids))
            })
    } else {
        None
    };
    let mut scored = items
        .into_iter()
        .filter_map(|item| {
            category_briefing_score(context, category_id, class_ids.as_ref(), &item)
                .map(|score| (item, score))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.timestamp.cmp(&left.0.timestamp))
    });

    let mut selected = scored
        .iter()
        .filter(|(_, score)| *score >= 130)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();

    if selected.len() < 3 {
        selected = scored.into_iter().take(3).map(|(item, _)| item).collect();
    }

    selected
}

fn select_public_briefing_items(context: &NewsTabContext, items: Vec<NewsItem>) -> Vec<NewsItem> {
    let mut scored = items
        .into_iter()
        .filter_map(|item| public_briefing_score(context, &item).map(|score| (item, score)))
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.timestamp.cmp(&left.0.timestamp))
    });

    let mut selected = scored
        .iter()
        .filter(|(_, score)| *score >= 110)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();

    if selected.len() < 4 {
        selected = scored.into_iter().take(4).map(|(item, _)| item).collect();
    }

    selected
}

struct EditorialStoryContext {
    /// Subject string passed to deck/block templates ("Apex Academy", "João Silva", …)
    subject: String,
    /// Broad kind of subject: "driver" | "team" | "category" | "paddock"
    subject_kind: String,
    /// Category string passed to deck/block templates ("Mazda MX-5 Rookie Cup", "o campeonato")
    category: String,
    /// Next-race label passed to deck templates ("Summit Point", "a próxima etapa")
    event_label: String,
    // --- payload fields ---
    entity_label: Option<String>,
    driver_label: Option<String>,
    team_label: Option<String>,
    category_label: Option<String>,
    race_label: Option<String>,
    next_race_label: Option<String>,
    next_race_date_label: Option<String>,
    driver_secondary_label: Option<String>,
    driver_position: Option<i32>,
    driver_points: Option<i32>,
    team_position: Option<i32>,
    team_points: Option<i32>,
    team_color_primary: Option<String>,
    team_color_secondary: Option<String>,
    team_public_presence_tier: Option<String>,
}

fn build_editorial_story_context(context: &NewsTabContext, item: &NewsItem) -> EditorialStoryContext {
    let category_label = item.categoria_nome.clone().or_else(|| {
        item.categoria_id
            .as_ref()
            .and_then(|id| context.category_names.get(id).cloned())
    });
    let team_label = item
        .team_id
        .as_ref()
        .and_then(|id| context.team_names.get(id).cloned());
    let driver_label = item
        .driver_id
        .as_ref()
        .and_then(|id| context.driver_names.get(id).cloned());
    let entity_label = team_label.clone().or_else(|| driver_label.clone());
    let race_label = item
        .categoria_id
        .as_ref()
        .and_then(|cat| item.rodada.map(|r| format!("{cat}:{r}")))
        .and_then(|key| context.race_labels.get(&key).cloned());
    let next_race_info = item
        .categoria_id
        .as_ref()
        .and_then(|cat| context.next_race_by_category.get(cat));
    let next_race_label = next_race_info.map(|r| r.label.clone());
    let next_race_date_label = next_race_info.map(|r| r.date_label.clone());
    let event_label = next_race_info
        .map(|r| r.label.clone())
        .unwrap_or_else(|| "a próxima etapa".to_string());
    let driver_secondary_label = item
        .driver_id_secondary
        .as_ref()
        .and_then(|id| context.driver_names.get(id).cloned());
    let driver_key = item
        .categoria_id
        .as_ref()
        .zip(item.driver_id.as_ref())
        .map(|(cat, drv)| format!("{cat}:{drv}"));
    let driver_position = driver_key
        .as_deref()
        .and_then(|k| context.driver_positions.get(k).copied());
    let driver_points = driver_key
        .as_deref()
        .and_then(|k| context.driver_points.get(k).copied());
    let team_position = item
        .team_id
        .as_ref()
        .and_then(|id| context.team_positions.get(id).copied());
    let team_points = item
        .team_id
        .as_ref()
        .and_then(|id| context.team_points.get(id).copied());
    let (team_color_primary, team_color_secondary) = item
        .team_id
        .as_ref()
        .and_then(|id| context.team_colors.get(id))
        .map(|(p, s)| (Some(p.clone()), Some(s.clone())))
        .unwrap_or((None, None));
    let team_public_presence_tier = item
        .team_id
        .as_ref()
        .and_then(|id| context.team_public_presence.get(id).cloned());

    let subject = entity_label
        .as_deref()
        .or(driver_label.as_deref())
        .or(team_label.as_deref())
        .or(category_label.as_deref())
        .unwrap_or("o paddock")
        .to_string();
    let subject_kind = if item.driver_id.is_some() {
        "driver"
    } else if item.team_id.is_some() {
        "team"
    } else if item.categoria_id.is_some() {
        "category"
    } else {
        "paddock"
    }
    .to_string();
    let category = category_label
        .as_deref()
        .unwrap_or("o campeonato")
        .to_string();

    EditorialStoryContext {
        subject,
        subject_kind,
        category,
        event_label,
        entity_label,
        driver_label,
        team_label,
        category_label,
        race_label,
        next_race_label,
        next_race_date_label,
        driver_secondary_label,
        driver_position,
        driver_points,
        team_position,
        team_points,
        team_color_primary,
        team_color_secondary,
        team_public_presence_tier,
    }
}

fn build_stories(context: &NewsTabContext, items: Vec<NewsItem>) -> Vec<NewsTabStory> {
    items
        .into_iter()
        .map(|item| {
            let sc = build_editorial_story_context(context, &item);
            let meta_label = build_meta_label(&item);
            let time_label = build_story_time_label(context, &item);
            let news_type = item.tipo.as_str().to_string();
            let importance = item.importancia.as_str().to_string();
            let importance_label = importance_label(&item.importancia).to_string();
            let accent_tone = story_accent(&item.importancia, &item.tipo).to_string();
            let editorial_type = classify_editorial_story_type(&item);
            let (deck_variation, block_variations) = editorial_slot_indexes(&item, editorial_type);
            let headline = item.titulo.clone();
            let deck = build_story_deck(
                editorial_type,
                deck_variation,
                &sc.subject,
                &sc.category,
                &sc.event_label,
            );
            let block_labels = editorial_block_labels(editorial_type);
            let editorial_extras = EditorialExtras {
                driver_position: sc.driver_position,
                team_position: sc.team_position,
                driver_secondary_label: sc.driver_secondary_label.as_deref(),
                preseason_week: item.semana_pretemporada,
                presence_tier: sc.team_public_presence_tier.as_deref(),
                subject_is_team: sc.subject_kind == "team",
                event_label: &sc.event_label,
            };
            let block_texts = build_editorial_blocks(
                editorial_type,
                block_variations,
                &item,
                &sc.subject,
                &sc.category,
                &editorial_extras,
            );
            let blocks = block_labels
                .into_iter()
                .zip(block_texts)
                .map(|(label, text)| NewsTabStoryBlock {
                    label: label.to_string(),
                    text,
                })
                .collect::<Vec<_>>();
            let body_text = blocks
                .iter()
                .map(|block| format!("{}: {}", block.label, block.text))
                .collect::<Vec<_>>()
                .join(" ");

            NewsTabStory {
                id: item.id,
                icon: item.icone,
                title: headline.clone(),
                headline,
                summary: deck.clone(),
                deck,
                body_text,
                blocks,
                news_type,
                importance,
                importance_label,
                category_label: sc.category_label,
                meta_label,
                time_label,
                entity_label: sc.entity_label,
                driver_label: sc.driver_label,
                team_label: sc.team_label,
                race_label: sc.race_label,
                accent_tone,
                driver_id: item.driver_id,
                team_id: item.team_id,
                round: item.rodada,
                original_text: Some(item.texto),
                preseason_week: item.semana_pretemporada,
                season_number: item.temporada,
                driver_id_secondary: item.driver_id_secondary,
                driver_secondary_label: sc.driver_secondary_label,
                driver_position: sc.driver_position,
                driver_points: sc.driver_points,
                team_position: sc.team_position,
                team_points: sc.team_points,
                team_color_primary: sc.team_color_primary,
                team_color_secondary: sc.team_color_secondary,
                next_race_label: sc.next_race_label,
                next_race_date_label: sc.next_race_date_label,
                team_public_presence_tier: sc.team_public_presence_tier,
            }
        })
        .collect()
}

fn build_hero(
    context: &NewsTabContext,
    scope_type: &str,
    scope_id: &str,
    scope_label: &str,
    _scope_class: Option<&str>,
    primary_filter: Option<&str>,
) -> NewsTabHero {
    let next_race = if scope_type == "category" {
        next_race_for_category(context, scope_id)
    } else {
        None
    };
    let badge = if primary_filter.is_none() && scope_type == "famous" {
        "Briefing Publico".to_string()
    } else if primary_filter.is_none() {
        next_race
            .as_ref()
            .map(|(track_name, round)| format!("Briefing R{} | {}", round, track_name))
            .unwrap_or_else(|| "Briefing da Rodada".to_string())
    } else if primary_filter == Some("Mercado") {
        "Mercado Aquecido".to_string()
    } else if scope_type == "famous" {
        "Em Alta".to_string()
    } else if context.career.season.rodada_atual <= 1 {
        "Inicio de Temporada".to_string()
    } else if context.career.season.rodada_atual >= context.career.season.total_rodadas {
        "Reta Final".to_string()
    } else {
        format!(
            "Rodada {}/{}",
            context.career.season.rodada_atual, context.career.season.total_rodadas
        )
    };

    let subtitle = match (scope_type, primary_filter) {
        ("famous", Some("Pilotos")) => {
            "Os nomes mais quentes do paddock neste momento.".to_string()
        }
        ("famous", Some("Equipes")) => {
            "As estruturas com maior presenca publica do campeonato.".to_string()
        }
        ("famous", Some("Mercado")) => {
            "Quem domina a conversa publica nas movimentacoes do grid.".to_string()
        }
        ("famous", _) => {
            "Quem domina a conversa agora e leva atencao para o proximo momento do campeonato."
                .to_string()
        }
        (_, Some("Corridas")) => {
            format!("Etapas e acontecimentos que estao moldando {scope_label}.")
        }
        (_, Some("Mercado")) => format!("Rumores e movimentos que agitam {scope_label}."),
        (_, Some("Equipes")) => {
            format!("O paddock institucional e esportivo que sustenta {scope_label}.")
        }
        (_, Some("Pilotos")) => format!("Os protagonistas e as tensoes centrais de {scope_label}."),
        _ => {
            if let Some((track_name, _)) = next_race {
                format!(
                    "O que segue vivo no paddock antes de {} e o que pesa para a proxima largada em {scope_label}.",
                    track_name
                )
            } else {
                format!("O que segue no ar antes da proxima rodada em {scope_label}.")
            }
        }
    };

    NewsTabHero {
        section_label: "Central de Notícias".to_string(),
        title: "Panorama do Campeonato".to_string(),
        subtitle,
        badge,
        badge_tone: if scope_type == "famous" {
            "gold".to_string()
        } else {
            "blue".to_string()
        },
    }
}

fn normalize_scope_type(value: &str) -> &str {
    if value.eq_ignore_ascii_case("famous") {
        "famous"
    } else {
        "category"
    }
}

fn normalize_scope_id(scope_type: &str, requested: &str, fallback_category: &str) -> String {
    if scope_type == "famous" {
        "mais_famosos".to_string()
    } else if categories::get_category_config(requested).is_some() {
        requested.to_string()
    } else {
        fallback_category.to_string()
    }
}

fn normalize_scope_class(scope_id: &str, requested: Option<&str>) -> Option<String> {
    let requested = requested.map(str::trim).filter(|value| !value.is_empty())?;
    let category = categories::get_category_config(scope_id)?;
    category
        .classes
        .iter()
        .find(|class_info| class_info.class_name.eq_ignore_ascii_case(requested))
        .map(|class_info| class_info.class_name.to_string())
}

fn normalize_primary_filter(scope_type: &str, requested: Option<&str>) -> Option<String> {
    let requested = requested.map(str::trim).filter(|value| !value.is_empty())?;
    let allowed = if scope_type == "famous" {
        &FAMOUS_FILTER_IDS[..]
    } else {
        &PRIMARY_FILTER_IDS[..]
    };
    allowed
        .iter()
        .find(|value| value.eq_ignore_ascii_case(requested))
        .map(|value| (*value).to_string())
}

fn story_belongs_to_category(
    item: &NewsItem,
    category_id: &str,
    scope_class: Option<&str>,
    driver_ids: &HashSet<String>,
    team_ids: &HashSet<String>,
) -> bool {
    let matches_scope_members = item_mentions_any_driver(item, driver_ids)
        || item
            .team_id
            .as_ref()
            .map(|value| team_ids.contains(value))
            .unwrap_or(false);

    if scope_class.is_some() {
        matches_scope_members
    } else {
        item.categoria_id.as_deref() == Some(category_id) || matches_scope_members
    }
}

fn story_is_famous(
    item: &NewsItem,
    famous_driver_ids: &HashSet<String>,
    famous_team_ids: &HashSet<String>,
) -> bool {
    item_mentions_any_driver(item, famous_driver_ids)
        || item
            .team_id
            .as_ref()
            .map(|value| famous_team_ids.contains(value))
            .unwrap_or(false)
        || matches!(
            item.importancia,
            NewsImportance::Alta | NewsImportance::Destaque
        )
}

fn story_matches_primary_filter(
    _context: &NewsTabContext,
    _scope_id: &str,
    item: &NewsItem,
    primary_filter: Option<&str>,
) -> bool {
    match primary_filter {
        Some("Mercado") => matches!(item.tipo, NewsType::Mercado | NewsType::PreTemporada),
        Some("Corridas") => item.rodada.unwrap_or(0) > 0,
        Some("Equipes") => {
            item.team_id.is_some()
                || item.driver_id.is_some()
                || item.driver_id_secondary.is_some()
                || matches!(item.tipo, NewsType::Hierarquia)
        }
        Some("Pilotos") => {
            item.driver_id.is_some()
                || item.driver_id_secondary.is_some()
                || matches!(item.tipo, NewsType::Rivalidade | NewsType::Incidente)
        }
        _ => true,
    }
}

fn item_mentions_driver(item: &NewsItem, driver_id: &str) -> bool {
    item.driver_id.as_deref() == Some(driver_id)
        || item.driver_id_secondary.as_deref() == Some(driver_id)
}

fn item_mentions_any_driver(item: &NewsItem, driver_ids: &HashSet<String>) -> bool {
    item.driver_id
        .as_ref()
        .map(|value| driver_ids.contains(value))
        .unwrap_or(false)
        || item
            .driver_id_secondary
            .as_ref()
            .map(|value| driver_ids.contains(value))
            .unwrap_or(false)
}

fn category_briefing_score(
    context: &NewsTabContext,
    scope_id: &str,
    class_ids: Option<&(HashSet<String>, HashSet<String>)>,
    item: &NewsItem,
) -> Option<i32> {
    let mut score = match item.tipo {
        NewsType::FramingSazonal => 220,
        NewsType::PreTemporada => 190,
        NewsType::Rivalidade => 170,
        NewsType::Incidente => 160,
        NewsType::Corrida => 135,
        NewsType::Hierarquia => 115,
        NewsType::Mercado => 70,
        _ if matches!(
            item.importancia,
            NewsImportance::Alta | NewsImportance::Destaque
        ) =>
        {
            85
        }
        _ => 0,
    };

    if score == 0 {
        return None;
    }

    if item.categoria_id.as_deref() == Some(scope_id) {
        score += 18;
    }

    if let Some((team_ids, driver_ids)) = class_ids {
        if item_mentions_any_driver(item, driver_ids)
            || item
                .team_id
                .as_ref()
                .map(|team_id| team_ids.contains(team_id))
                .unwrap_or(false)
        {
            score += 16;
        }
    }

    let current_round = context.career.season.rodada_atual.max(1);
    let previous_round = current_round.saturating_sub(1);
    if item.rodada == Some(current_round) {
        score += 28;
    } else if previous_round > 0 && item.rodada == Some(previous_round) {
        score += 22;
    } else if item.tipo == NewsType::Corrida {
        score -= 26;
    }

    if matches!(item.tipo, NewsType::Mercado) && item.rodada.unwrap_or(0) <= 0 {
        score -= 24;
    }

    score += importance_rank(&item.importancia) * 9;
    score += freshness_bonus(context.newest_timestamp, item.timestamp);
    Some(score)
}

fn public_briefing_score(context: &NewsTabContext, item: &NewsItem) -> Option<i32> {
    let mut score = match item.tipo {
        NewsType::FramingSazonal => 170,
        NewsType::Rivalidade => 155,
        NewsType::Mercado => 145,
        NewsType::Corrida => 125,
        _ if matches!(
            item.importancia,
            NewsImportance::Alta | NewsImportance::Destaque
        ) =>
        {
            110
        }
        _ => 0,
    };

    if score == 0 {
        return None;
    }

    score += importance_rank(&item.importancia) * 8;
    score += freshness_bonus(context.newest_timestamp, item.timestamp);
    Some(score)
}

fn next_race_for_category(context: &NewsTabContext, category_id: &str) -> Option<(String, i32)> {
    get_calendar_for_category_in_base_dir(&context.base_dir, &context.career_id, category_id)
        .ok()?
        .into_iter()
        .find(|race| race.rodada >= context.career.season.rodada_atual)
        .map(|race| (race.track_name, race.rodada))
}

fn story_matches_context(
    context: &NewsTabContext,
    item: &NewsItem,
    selection: &ContextSelection,
) -> bool {
    match selection.kind.as_str() {
        "race" => context
            .race_rounds
            .get(&selection.id)
            .map(|round| item.rodada == Some(*round))
            .unwrap_or(false),
        "team" => {
            item.team_id.as_deref() == Some(selection.id.as_str())
                || context
                    .team_driver_ids
                    .get(&selection.id)
                    .map(|ids| {
                        ids.iter()
                            .any(|driver_id| item_mentions_driver(item, driver_id))
                    })
                    .unwrap_or(false)
        }
        "driver" => item_mentions_driver(item, selection.id.as_str()),
        "rivalry" => {
            let ids: Vec<&str> = selection.id.split('|').collect();
            ids.iter()
                .any(|driver_id| item_mentions_driver(item, driver_id))
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::Duration;

    use super::{get_news_tab_bootstrap_in_base_dir, get_news_tab_snapshot_in_base_dir};
    use crate::commands::career::{
        create_career_in_base_dir, get_calendar_for_category_in_base_dir,
    };
    use crate::commands::career_types::{CreateCareerInput, NewsTabSnapshotRequest};
    use crate::commands::news_editorial::{
        build_editorial_blocks, build_story_deck, editorial_block_labels, editorial_variant_index,
        EditorialExtras, EditorialStoryType, EDITORIAL_BLOCK_VARIANT_COUNT,
        EDITORIAL_DECK_VARIANT_COUNT,
    };
    use crate::commands::news_helpers::{
        format_display_date_label, format_naive_date_label, parse_iso_date,
    };
    use crate::config::app_config::AppConfig;
    use crate::db::connection::Database;
    use crate::db::queries::news as news_queries;
    use crate::db::queries::seasons as season_queries;
    use crate::db::queries::teams as team_queries;
    use crate::news::{NewsImportance, NewsItem, NewsType};
    use serde_json::Value;

    #[test]
    fn test_news_tab_bootstrap_defaults_to_player_category_and_includes_famous_scope() {
        let base_dir = create_test_career_dir("news_bootstrap");
        let bootstrap =
            get_news_tab_bootstrap_in_base_dir(&base_dir, "career_001").expect("bootstrap");
        let bootstrap_json = serde_json::to_value(&bootstrap).expect("bootstrap json");

        assert_eq!(bootstrap.default_scope_type, "category");
        assert_eq!(bootstrap.default_scope_id, "mazda_rookie");
        assert_eq!(
            bootstrap_json.get("default_primary_filter"),
            Some(&Value::Null)
        );
        assert!(bootstrap
            .scopes
            .iter()
            .any(|scope| scope.id == "mazda_rookie"));
        assert!(bootstrap
            .scopes
            .iter()
            .any(|scope| scope.id == "mais_famosos" && scope.special));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_for_race_focus_returns_round_chips_and_filtered_stories() {
        let base_dir = create_test_career_dir("news_snapshot_race_focus");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Corridas".to_string()),
                context_type: Some("race".to_string()),
                context_id: Some("R001".to_string()),
            },
        )
        .expect("snapshot");
        let snapshot_json = serde_json::to_value(&snapshot).expect("snapshot json");

        assert!(snapshot
            .contextual_filters
            .iter()
            .any(|chip| chip.id == "R001"));
        let stories = snapshot_json
            .get("stories")
            .and_then(|value| value.as_array())
            .expect("stories array");
        assert!(stories
            .iter()
            .all(|story| story.get("round") == Some(&Value::from(1))));
        assert_eq!(
            snapshot.scope_meta.primary_filter.as_deref(),
            Some("Corridas")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_market_builds_entity_context_filters_from_market_stories() {
        let base_dir = create_test_career_dir("news_snapshot_market_context");
        let team_id = current_team_id(&base_dir, "career_001");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Mercado".to_string()),
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");
        let snapshot_json = serde_json::to_value(&snapshot).expect("snapshot json");

        assert_eq!(snapshot.scope_meta.context_type, None);
        assert_eq!(snapshot.scope_meta.context_id, None);
        assert!(snapshot
            .contextual_filters
            .iter()
            .any(|chip| chip.id == team_id));
        let stories = snapshot_json
            .get("stories")
            .and_then(|value| value.as_array())
            .expect("stories array");
        assert!(stories.len() >= 2);
        assert!(stories.iter().all(|story| {
            matches!(
                story.get("news_type").and_then(|value| value.as_str()),
                Some("Mercado" | "PreTemporada")
            )
        }));
        assert!(stories.iter().any(|story| {
            story.get("title") == Some(&Value::from("A equipe do jogador observa reforcos"))
        }));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_market_requires_matching_context_type_and_id() {
        let base_dir = create_test_career_dir("news_snapshot_market_type_guard");
        let team_id = current_team_id(&base_dir, "career_001");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Mercado".to_string()),
                context_type: Some("driver".to_string()),
                context_id: Some(team_id),
            },
        )
        .expect("snapshot");

        assert_eq!(snapshot.scope_meta.context_type, None);
        assert_eq!(snapshot.scope_meta.context_id, None);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_driver_context_matches_secondary_driver_in_rivalry_story() {
        let base_dir = create_test_career_dir("news_snapshot_driver_secondary");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Pilotos".to_string()),
                context_type: Some("driver".to_string()),
                context_id: Some("P002".to_string()),
            },
        )
        .expect("snapshot");

        assert!(snapshot.stories.iter().any(|story| {
            story
                .title
                .contains("Thomas Baker e Kenji Sato entram em rota de colisao")
        }));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_for_famous_scope_uses_reduced_filters() {
        let base_dir = create_test_career_dir("news_snapshot_famous");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "famous".to_string(),
                scope_id: "mais_famosos".to_string(),
                scope_class: None,
                primary_filter: Some("Pilotos".to_string()),
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let filter_ids: Vec<&str> = snapshot
            .primary_filters
            .iter()
            .map(|filter| filter.id.as_str())
            .collect();
        assert_eq!(filter_ids, vec!["Pilotos", "Equipes", "Mercado"]);
        assert!(snapshot.scope_meta.is_special);
        assert!(!snapshot.contextual_filters.is_empty());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_without_filter_exposes_reader_story_list() {
        let base_dir = create_test_career_dir("news_snapshot_reader_list");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: None,
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");
        let snapshot_json = serde_json::to_value(&snapshot).expect("snapshot json");

        assert_eq!(
            snapshot_json
                .get("scope_meta")
                .and_then(|value| value.get("primary_filter")),
            Some(&Value::Null)
        );
        assert!(snapshot_json.get("cover_lead").is_none());
        assert!(snapshot_json.get("feed").is_none());
        assert!(snapshot_json
            .get("stories")
            .and_then(|value| value.as_array())
            .map(|stories| !stories.is_empty())
            .unwrap_or(false));
        let filter_ids: Vec<&str> = snapshot
            .primary_filters
            .iter()
            .map(|filter| filter.id.as_str())
            .collect();
        assert_eq!(
            filter_ids,
            vec!["Corridas", "Pilotos", "Equipes", "Mercado"]
        );
        assert!(snapshot.hero.subtitle.contains("proxima"));
        assert_eq!(snapshot.contextual_filters.len(), 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_builds_modular_story_fields_for_reader_cards() {
        let base_dir = create_test_career_dir("news_snapshot_modular_story_fields");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: None,
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let race_story = snapshot
            .stories
            .iter()
            .find(|story| story.title == "Abertura em Okayama esquenta o grid")
            .expect("race story");
        let race_labels: Vec<&str> = race_story
            .blocks
            .iter()
            .map(|block| block.label.as_str())
            .collect();

        assert_eq!(race_story.headline, "Abertura em Okayama esquenta o grid");
        assert_eq!(race_story.summary, race_story.deck);
        assert_eq!(race_story.blocks.len(), 3);
        assert_eq!(race_labels, vec!["Resumo", "Impacto", "Leitura"]);
        assert!(
            race_story
                .body_text
                .contains("Resumo:"),
            "legacy body_text should now mirror the modular blocks for compatibility",
        );

        let pilot_story = snapshot
            .stories
            .iter()
            .find(|story| story.title == "Thomas Baker e Kenji Sato entram em rota de colisao")
            .expect("pilot story");
        let pilot_labels: Vec<&str> = pilot_story
            .blocks
            .iter()
            .map(|block| block.label.as_str())
            .collect();

        assert_eq!(pilot_labels, vec!["Momento", "Pressão", "Sinal"]);
        assert!(
            pilot_story
                .deck
                .to_lowercase()
                .contains("mazda"),
            "deck should be regenerated from editorial context instead of reusing legacy prose",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_editorial_variant_index_reaches_expanded_ranges_per_slot() {
        let deck_indexes = (0..256)
            .map(|index| {
                editorial_variant_index(
                    &NewsItem {
                        id: format!("VAR{index:03}"),
                        tipo: NewsType::Corrida,
                        icone: "R".to_string(),
                        titulo: format!("Titulo {index}"),
                        texto: "Texto".to_string(),
                        rodada: Some(1),
                        semana_pretemporada: None,
                        temporada: 1,
                        categoria_id: Some("mazda_rookie".to_string()),
                        categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                        importancia: NewsImportance::Alta,
                        timestamp: index as i64,
                        driver_id: None,
                        driver_id_secondary: None,
                        team_id: None,
                    },
                    EditorialStoryType::Corrida,
                    "deck",
                    EDITORIAL_DECK_VARIANT_COUNT,
                )
            })
            .collect::<std::collections::HashSet<_>>();
        let block_indexes = (0..256)
            .map(|index| {
                editorial_variant_index(
                    &NewsItem {
                        id: format!("BLK{index:03}"),
                        tipo: NewsType::Corrida,
                        icone: "R".to_string(),
                        titulo: format!("Título {index}"),
                        texto: "Texto".to_string(),
                        rodada: Some(1),
                        semana_pretemporada: None,
                        temporada: 1,
                        categoria_id: Some("mazda_rookie".to_string()),
                        categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                        importancia: NewsImportance::Alta,
                        timestamp: index as i64,
                        driver_id: None,
                        driver_id_secondary: None,
                        team_id: None,
                    },
                    EditorialStoryType::Corrida,
                    "block-0",
                    EDITORIAL_BLOCK_VARIANT_COUNT,
                )
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(
            deck_indexes
                .iter()
                .all(|index| *index < EDITORIAL_DECK_VARIANT_COUNT),
            "deck variation range should stay below the declared deck limit",
        );
        assert!(
            deck_indexes.iter().any(|index| *index >= 12),
            "deck range should now reach well beyond the old ceiling",
        );
        assert!(
            block_indexes
                .iter()
                .all(|index| *index < EDITORIAL_BLOCK_VARIANT_COUNT),
            "block variation range should stay below the declared block limit",
        );
        assert!(
            block_indexes.iter().any(|index| *index >= 8),
            "block range should now reach the expanded block catalog",
        );
    }

    #[test]
    fn test_story_deck_has_multiple_variants_per_editorial_type() {
        let corrida_decks = (0..EDITORIAL_DECK_VARIANT_COUNT)
            .map(|variation| {
                build_story_deck(
                    EditorialStoryType::Corrida,
                    variation,
                    "Rodrigo",
                    "Mazda MX-5 Rookie Cup",
                    "Okayama",
                )
            })
            .collect::<std::collections::HashSet<_>>();
        let incidente_decks = (0..EDITORIAL_DECK_VARIANT_COUNT)
            .map(|variation| {
                build_story_deck(
                    EditorialStoryType::Incidente,
                    variation,
                    "Rodrigo",
                    "Mazda MX-5 Rookie Cup",
                    "Okayama",
                )
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(
            corrida_decks.len() >= 12,
            "race decks should no longer collapse into a single repeated sentence",
        );
        assert!(
            incidente_decks.len() >= 12,
            "incident decks should also have meaningful variety",
        );
    }

    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.match_indices(needle).count()
    }

    fn editorial_test_extras<'a>(event_label: &'a str) -> EditorialExtras<'a> {
        EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label,
        }
    }

    #[test]
    fn test_editorial_blocks_do_not_repeat_explicit_subject_or_event_across_story() {
        let item = NewsItem {
            id: "REPEAT001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let kinds = [
            EditorialStoryType::Corrida,
            EditorialStoryType::Incidente,
            EditorialStoryType::Piloto,
            EditorialStoryType::Equipe,
            EditorialStoryType::Mercado,
            EditorialStoryType::Estrutural,
        ];
        let subject = "RODRIGO-ANCHOR";
        let event_label = "OKAYAMA-ANCHOR";
        let extras = editorial_test_extras(event_label);

        for kind in kinds {
            for variation in 0..EDITORIAL_BLOCK_VARIANT_COUNT {
                let joined = build_editorial_blocks(
                    kind,
                    [
                        variation,
                        (variation + 3) % EDITORIAL_BLOCK_VARIANT_COUNT,
                        (variation + 7) % EDITORIAL_BLOCK_VARIANT_COUNT,
                    ],
                    &item,
                    subject,
                    "Mazda MX-5 Rookie Cup",
                    &extras,
                )
                .join(" ");

                assert!(
                    count_occurrences(&joined, subject) <= 2,
                    "subject anchor repeated too much for {kind:?} variation {variation}: {joined}",
                );
                assert!(
                    count_occurrences(&joined, event_label) <= 1,
                    "event anchor repeated too much for {kind:?} variation {variation}: {joined}",
                );
            }
        }
    }

    #[test]
    fn test_editorial_labels_use_accented_portuguese() {
        assert_eq!(
            editorial_block_labels(EditorialStoryType::Incidente),
            ["Ocorrido", "Consequência", "Estado"]
        );
        assert_eq!(
            editorial_block_labels(EditorialStoryType::Piloto),
            ["Momento", "Pressão", "Sinal"]
        );
        assert_eq!(
            editorial_block_labels(EditorialStoryType::Mercado),
            ["Movimento", "Impacto", "Próximo passo"]
        );
        assert_eq!(
            editorial_block_labels(EditorialStoryType::Estrutural),
            ["Mudança", "Efeito", "Panorama"]
        );
    }

    #[test]
    fn test_editorial_text_never_uses_raw_importance_label_as_broken_prose() {
        let item = NewsItem {
            id: "PROSE001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: NewsImportance::Baixa,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let broken_fragments = [
            "peso baixa",
            "choque baixa",
            "clima baixa",
            "sinal baixa",
            "panorama baixa",
            "momento baixa",
        ];

        for kind in [
            EditorialStoryType::Corrida,
            EditorialStoryType::Incidente,
            EditorialStoryType::Piloto,
            EditorialStoryType::Equipe,
            EditorialStoryType::Mercado,
            EditorialStoryType::Estrutural,
        ] {
            for variation in 0..EDITORIAL_DECK_VARIANT_COUNT {
                let deck = build_story_deck(
                    kind,
                    variation,
                    "Rodrigo",
                    "Mazda MX-5 Rookie Cup",
                    "Okayama",
                )
                .to_lowercase();

                for fragment in broken_fragments {
                    assert!(
                        !deck.contains(fragment),
                        "deck for {kind:?} variation {variation} should not contain broken prose fragment `{fragment}`: {deck}",
                    );
                }
            }

            for variation in 0..EDITORIAL_BLOCK_VARIANT_COUNT {
                let extras = editorial_test_extras("Okayama");
                let joined = build_editorial_blocks(
                    kind,
                    [
                        variation,
                        (variation + 1) % EDITORIAL_BLOCK_VARIANT_COUNT,
                        (variation + 2) % EDITORIAL_BLOCK_VARIANT_COUNT,
                    ],
                    &item,
                    "Rodrigo",
                    "Mazda MX-5 Rookie Cup",
                    &extras,
                )
                .join(" ")
                .to_lowercase();

                for fragment in broken_fragments {
                    assert!(
                        !joined.contains(fragment),
                        "blocks for {kind:?} variation {variation} should not contain broken prose fragment `{fragment}`: {joined}",
                    );
                }
            }
        }
    }

    #[test]
    fn test_editorial_blocks_use_explicit_subject_instead_of_generic_aliases() {
        let item = NewsItem {
            id: "SUBJECT001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let subject = "Carlos Mendes";
        let extras = editorial_test_extras("Okayama");

        let corrida_blocks = build_editorial_blocks(
            EditorialStoryType::Corrida,
            [0, 0, 2],
            &item,
            subject,
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert!(corrida_blocks[2].contains(subject));
        assert!(!corrida_blocks[2].contains("o nome em foco"));

        let incidente_blocks = build_editorial_blocks(
            EditorialStoryType::Incidente,
            [0, 0, 3],
            &item,
            subject,
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert!(incidente_blocks[2].contains(subject));
        assert!(!incidente_blocks[2].contains("o nome em foco"));

        let piloto_pressao_blocks = build_editorial_blocks(
            EditorialStoryType::Piloto,
            [0, 2, 0],
            &item,
            subject,
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert!(piloto_pressao_blocks[1].contains(subject));
        assert!(!piloto_pressao_blocks[1].contains("o piloto"));

        let piloto_sinal_blocks = build_editorial_blocks(
            EditorialStoryType::Piloto,
            [0, 0, 3],
            &item,
            subject,
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert!(piloto_sinal_blocks[2].contains(subject));
        assert!(!piloto_sinal_blocks[2].contains("o piloto"));
    }

    #[test]
    fn test_corrida_leitura_variants_do_not_name_event_label_explicitly() {
        let item = NewsItem {
            id: "EVENT001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let event_label = "OKAYAMA-ANCHOR";
        let extras = editorial_test_extras(event_label);

        for variation in 0..EDITORIAL_BLOCK_VARIANT_COUNT {
            let blocks = build_editorial_blocks(
                EditorialStoryType::Corrida,
                [0, 0, variation],
                &item,
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
                &extras,
            );

            assert!(
                !blocks[2].contains(event_label),
                "corrida leitura variation {variation} should avoid explicit event label: {}",
                blocks[2],
            );
        }
    }

    #[test]
    fn test_editorial_summary_variants_do_not_treat_verbal_headline_as_subject() {
        let item = NewsItem {
            id: "HEADLINE001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Carlos Mendes assume a lideranca em Okayama".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let title = item.titulo.clone();
        let extras = editorial_test_extras("Okayama");
        let banned_suffixes = [
            " reorganiza",
            " mudou",
            " abre",
            " empurra",
            " muda",
            " nao fecha",
            " transforma",
            " faz",
            " coloca",
            " aponta",
            " reordena",
            " desloca",
            " deixa claro",
            " devolve",
            " torna dificil",
            " enquadra",
            " resume",
            " traz",
            " recoloca",
            " mostra",
            " da rosto",
            " tira",
            " funciona",
            " expoe",
            " marca",
            " organiza",
        ];

        for kind in [
            EditorialStoryType::Corrida,
            EditorialStoryType::Piloto,
            EditorialStoryType::Equipe,
            EditorialStoryType::Mercado,
            EditorialStoryType::Estrutural,
        ] {
            for variation in 0..EDITORIAL_BLOCK_VARIANT_COUNT {
                let blocks = build_editorial_blocks(
                    kind,
                    [variation, 0, 0],
                    &item,
                    "Carlos Mendes",
                    "Mazda MX-5 Rookie Cup",
                    &extras,
                );

                for suffix in banned_suffixes {
                    let bad_fragment = format!("{title}{suffix}");
                    assert!(
                        !blocks[0].contains(&bad_fragment),
                        "summary block for {kind:?} variation {variation} should not use verbal headline as subject: {}",
                        blocks[0],
                    );
                }
            }
        }
    }

    #[test]
    fn test_news_tab_snapshot_without_filter_prioritizes_briefing_story_over_generic_market_dump() {
        let base_dir = create_test_career_dir("news_snapshot_briefing_priority");
        seed_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: None,
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        assert_eq!(
            snapshot.stories.first().map(|story| story.title.as_str()),
            Some("Pressao sobe para a proxima etapa em Okayama")
        );
        assert!(
            snapshot
                .stories
                .iter()
                .all(|story| story.title != "Arquivo do mercado ainda repercute no grid"),
            "briefing edition should not devolve into a generic all-news dump",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_time_label_uses_round_and_career_date_for_race_news() {
        let base_dir = create_test_career_dir("news_time_label_round");
        seed_news_items(&base_dir, "career_001");
        let expected_round_label = get_calendar_for_category_in_base_dir(
            &base_dir,
            "career_001",
            "mazda_rookie",
        )
        .expect("calendar")
        .into_iter()
        .find(|entry| entry.rodada == 1)
        .and_then(|entry| format_display_date_label(&entry.display_date))
        .expect("round 1 date");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Corridas".to_string()),
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let story = snapshot
            .stories
            .iter()
            .find(|story| story.title == "Abertura em Okayama esquenta o grid")
            .expect("race story");

        assert_eq!(story.time_label, format!("Rodada 1 · {expected_round_label}"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_time_label_uses_preseason_week_and_derived_date() {
        let base_dir = create_test_career_dir("news_time_label_preseason");
        seed_news_items(&base_dir, "career_001");
        seed_preseason_news_items(&base_dir, "career_001");
        let first_round_date = get_calendar_for_category_in_base_dir(
            &base_dir,
            "career_001",
            "mazda_rookie",
        )
        .expect("calendar")
        .into_iter()
        .find(|entry| entry.rodada == 1)
        .and_then(|entry| parse_iso_date(&entry.display_date))
        .expect("round 1 date");
        let expected_preseason_date = first_round_date - Duration::weeks(1);

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "mazda_rookie".to_string(),
                scope_class: None,
                primary_filter: Some("Mercado".to_string()),
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let story = snapshot
            .stories
            .iter()
            .find(|story| story.title == "Semana 2 agita o mercado da Mazda")
            .expect("preseason story");

        assert_eq!(
            story.time_label,
            format!(
                "Pre-temporada Semana 2 · {}",
                format_naive_date_label(expected_preseason_date)
            )
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_shared_scope_class_filters_everything_to_the_selected_production_class()
    {
        let base_dir = create_test_career_dir("news_snapshot_production_scope_class");
        seed_shared_scope_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "production_challenger".to_string(),
                scope_class: Some("mazda".to_string()),
                primary_filter: None,
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let story_titles: Vec<&str> = snapshot
            .stories
            .iter()
            .map(|story| story.title.as_str())
            .collect();

        assert!(
            story_titles
                .iter()
                .any(|title| title.contains("Mazda lidera o ritmo da Production")),
            "o briefing precisa manter a historia da classe Mazda",
        );
        assert!(
            story_titles.iter().all(|title| !title.contains("BMW")),
            "nenhuma historia da classe BMW deve sobreviver no recorte Mazda",
        );
        assert_eq!(snapshot.scope_meta.scope_id, "production_challenger");
        assert_eq!(snapshot.scope_meta.scope_class.as_deref(), Some("mazda"));
        assert!(
            snapshot.scope_meta.scope_label.contains("Mazda"),
            "o label ativo precisa refletir a classe compartilhada",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_news_tab_snapshot_shared_scope_class_limits_context_filters_to_the_selected_class() {
        let base_dir = create_test_career_dir("news_snapshot_production_class_filters");
        seed_shared_scope_news_items(&base_dir, "career_001");

        let snapshot = get_news_tab_snapshot_in_base_dir(
            &base_dir,
            "career_001",
            NewsTabSnapshotRequest {
                scope_type: "category".to_string(),
                scope_id: "production_challenger".to_string(),
                scope_class: Some("mazda".to_string()),
                primary_filter: Some("Equipes".to_string()),
                context_type: None,
                context_id: None,
            },
        )
        .expect("snapshot");

        let filter_labels: Vec<&str> = snapshot
            .contextual_filters
            .iter()
            .map(|filter| filter.label.as_str())
            .collect();

        assert!(
            !filter_labels.is_empty(),
            "o filtro contextual de equipes precisa continuar populado",
        );
        assert!(
            filter_labels
                .iter()
                .all(|label| !label.to_ascii_lowercase().contains("bmw")),
            "equipes BMW nao devem aparecer no recorte Mazda",
        );
        assert!(
            snapshot
                .stories
                .iter()
                .all(|story| !story.title.contains("BMW")),
            "historias da lista tambem devem respeitar o mesmo recorte",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    fn current_team_id(base_dir: &std::path::Path, career_id: &str) -> String {
        super::load_career_in_base_dir(base_dir, career_id)
            .expect("career")
            .player_team
            .id
    }

    fn current_team_by_class(
        base_dir: &std::path::Path,
        career_id: &str,
        category_id: &str,
        class_name: &str,
    ) -> String {
        let config = AppConfig::load_or_default(base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        team_queries::get_teams_by_category_and_class(&db.conn, category_id, class_name)
            .expect("teams by class")
            .into_iter()
            .next()
            .expect("team in class")
            .id
    }

    fn seed_news_items(base_dir: &std::path::Path, career_id: &str) {
        let config = AppConfig::load_or_default(base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let team_id = current_team_id(base_dir, career_id);

        news_queries::insert_news_batch(
            &db.conn,
            &vec![
                NewsItem {
                    id: "NT000".to_string(),
                    tipo: NewsType::FramingSazonal,
                    icone: "F".to_string(),
                    titulo: "Pressao sobe para a proxima etapa em Okayama".to_string(),
                    texto: "A disputa chega aquecida para a proxima rodada, com tensao crescente entre os nomes do topo.".to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 275,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                NewsItem {
                    id: "NT001".to_string(),
                    tipo: NewsType::Corrida,
                    icone: "R".to_string(),
                    titulo: "Abertura em Okayama esquenta o grid".to_string(),
                    texto: "Uma largada intensa abriu a temporada com disputa em toda a reta.".to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 100,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: Some(team_id.clone()),
                },
                NewsItem {
                    id: "NT002".to_string(),
                    tipo: NewsType::Mercado,
                    icone: "M".to_string(),
                    titulo: "A equipe do jogador observa reforcos".to_string(),
                    texto: "O paddock comenta uma movimentacao de mercado ao redor da equipe.".to_string(),
                    rodada: Some(2),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Destaque,
                    timestamp: 200,
                    driver_id: Some("P002".to_string()),
                    driver_id_secondary: None,
                    team_id: Some(team_id),
                },
                NewsItem {
                    id: "NT002B".to_string(),
                    tipo: NewsType::Mercado,
                    icone: "M".to_string(),
                    titulo: "Arquivo do mercado ainda repercute no grid".to_string(),
                    texto: "Um rumor antigo segue circulando, mas ja sem impacto claro sobre a proxima corrida.".to_string(),
                    rodada: Some(0),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Destaque,
                    timestamp: 260,
                    driver_id: Some("P002".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                NewsItem {
                    id: "NT003".to_string(),
                    tipo: NewsType::Rivalidade,
                    icone: "V".to_string(),
                    titulo: "Thomas Baker e Kenji Sato entram em rota de colisao".to_string(),
                    texto: "O duelo direto em pista comeca a definir o tom do campeonato.".to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 300,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: Some("P002".to_string()),
                    team_id: None,
                },
            ],
        )
        .expect("insert news");
    }

    fn seed_shared_scope_news_items(base_dir: &std::path::Path, career_id: &str) {
        let config = AppConfig::load_or_default(base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let mazda_team_id = current_team_by_class(
            base_dir,
            career_id,
            "production_challenger",
            "mazda",
        );
        let bmw_team_id = current_team_by_class(
            base_dir,
            career_id,
            "production_challenger",
            "bmw",
        );

        news_queries::insert_news_batch(
            &db.conn,
            &vec![
                NewsItem {
                    id: "SP001".to_string(),
                    tipo: NewsType::FramingSazonal,
                    icone: "P".to_string(),
                    titulo: "Mazda lidera o ritmo da Production".to_string(),
                    texto: "A ala Mazda chega forte ao fim de semana e puxa a conversa do paddock."
                        .to_string(),
                    rodada: Some(2),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("production_challenger".to_string()),
                    categoria_nome: Some("Production Car Challenger".to_string()),
                    importancia: NewsImportance::Destaque,
                    timestamp: 320,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: Some(mazda_team_id.clone()),
                },
                NewsItem {
                    id: "SP002".to_string(),
                    tipo: NewsType::Corrida,
                    icone: "B".to_string(),
                    titulo: "BMW domina o treino da Production".to_string(),
                    texto: "A frente BMW abriu vantagem no treino compartilhado.".to_string(),
                    rodada: Some(2),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("production_challenger".to_string()),
                    categoria_nome: Some("Production Car Challenger".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 310,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: Some(bmw_team_id.clone()),
                },
                NewsItem {
                    id: "SP003".to_string(),
                    tipo: NewsType::Mercado,
                    icone: "M".to_string(),
                    titulo: "Equipe Mazda acelera conversa de mercado na Production".to_string(),
                    texto: "A classe Mazda discute reforcos antes da proxima etapa multiclasses."
                        .to_string(),
                    rodada: Some(2),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("production_challenger".to_string()),
                    categoria_nome: Some("Production Car Challenger".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 300,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: Some(mazda_team_id),
                },
            ],
        )
        .expect("insert shared scope news");
    }

    fn seed_preseason_news_items(base_dir: &std::path::Path, career_id: &str) {
        let config = AppConfig::load_or_default(base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");

        news_queries::insert_news_batch(
            &db.conn,
            &vec![NewsItem {
                id: "NTP001".to_string(),
                tipo: NewsType::PreTemporada,
                icone: "P".to_string(),
                titulo: "Semana 2 agita o mercado da Mazda".to_string(),
                texto: "A segunda semana da pre-temporada acelera rumores e definicoes no paddock."
                    .to_string(),
                rodada: None,
                semana_pretemporada: Some(2),
                temporada: season.numero,
                categoria_id: Some("mazda_rookie".to_string()),
                categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                importancia: NewsImportance::Alta,
                timestamp: 450,
                driver_id: None,
                driver_id_secondary: None,
                team_id: None,
            }],
        )
        .expect("insert preseason news");
    }

    fn create_test_career_dir(label: &str) -> std::path::PathBuf {
        let base_dir = unique_test_dir(label);
        fs::create_dir_all(&base_dir).expect("base dir");

        let input = CreateCareerInput {
            player_name: "Joao Silva".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };

        let _ = create_career_in_base_dir(&base_dir, input).expect("career should be created");
        base_dir
    }

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("iracerapp_news_tab_{label}_{nanos}"))
    }
}
