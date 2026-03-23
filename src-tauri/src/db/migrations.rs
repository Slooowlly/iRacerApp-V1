use rusqlite::Connection;

use crate::db::connection::DbError;

// ── Versão atual do schema ────────────────────────────────────────────────────

const CURRENT_VERSION: u32 = 12;

// ── API pública ───────────────────────────────────────────────────────────────

/// Aplica todas as migrações num banco novo (versão 0 → CURRENT_VERSION).
pub fn run_all(conn: &Connection) -> Result<(), DbError> {
    migrate_v1(conn)?;
    migrate_v2(conn)?;
    migrate_v3(conn)?;
    migrate_v4(conn)?;
    migrate_v5(conn)?;
    migrate_v6(conn)?;
    migrate_v7(conn)?;
    migrate_v8(conn)?;
    migrate_v9(conn)?;
    migrate_v10(conn)?;
    migrate_v11(conn)?;
    migrate_v12(conn)?;
    set_schema_version(conn, CURRENT_VERSION)?;
    Ok(())
}

/// Aplica apenas as migrações pendentes num banco existente.
pub fn run_pending(conn: &Connection) -> Result<(), DbError> {
    let version = get_schema_version(conn).unwrap_or(0);
    if version < 1 {
        migrate_v1(conn)?;
        set_schema_version(conn, 1)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
        set_schema_version(conn, 2)?;
    }
    if version < 3 {
        migrate_v3(conn)?;
        set_schema_version(conn, 3)?;
    }
    if version < 4 {
        migrate_v4(conn)?;
        set_schema_version(conn, 4)?;
    }
    if version < 5 {
        migrate_v5(conn)?;
        set_schema_version(conn, 5)?;
    }
    if version < 6 {
        migrate_v6(conn)?;
        set_schema_version(conn, 6)?;
    }
    if version < 7 {
        migrate_v7(conn)?;
        set_schema_version(conn, 7)?;
    }
    if version < 8 {
        migrate_v8(conn)?;
        set_schema_version(conn, 8)?;
    }
    if version < 9 {
        migrate_v9(conn)?;
        set_schema_version(conn, 9)?;
    }
    if version < 10 {
        migrate_v10(conn)?;
        set_schema_version(conn, 10)?;
    }
    if version < 11 {
        migrate_v11(conn)?;
        set_schema_version(conn, 11)?;
    }
    if version < 12 {
        migrate_v12(conn)?;
        set_schema_version(conn, 12)?;
    }
    Ok(())
}

// ── Helpers de versão ─────────────────────────────────────────────────────────

pub fn get_schema_version(conn: &Connection) -> Result<u32, DbError> {
    // A tabela meta pode não existir ainda num banco vazio.
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='meta'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return Ok(0);
    }

    conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .map(|v| v.parse::<u32>().unwrap_or(0))
    .map_err(DbError::Sqlite)
}

fn set_schema_version(conn: &Connection, version: u32) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
        rusqlite::params![version.to_string()],
    )?;
    Ok(())
}

// ── Migração v1 — schema completo ─────────────────────────────────────────────

fn migrate_v1(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(MIGRATION_V1_DDL)?;
    seed_meta(conn)?;
    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<(), DbError> {
    ensure_column(conn, "teams", "nome_curto", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(
        conn,
        "teams",
        "cor_primaria",
        "TEXT NOT NULL DEFAULT '#FFFFFF'",
    )?;
    ensure_column(
        conn,
        "teams",
        "cor_secundaria",
        "TEXT NOT NULL DEFAULT '#000000'",
    )?;
    ensure_column(
        conn,
        "teams",
        "pais_sede",
        "TEXT NOT NULL DEFAULT 'Unknown'",
    )?;
    ensure_column(
        conn,
        "teams",
        "ano_fundacao",
        "INTEGER NOT NULL DEFAULT 2024",
    )?;
    ensure_column(conn, "teams", "ativa", "INTEGER NOT NULL DEFAULT 1")?;
    ensure_column(conn, "teams", "marca", "TEXT")?;
    ensure_column(conn, "teams", "classe", "TEXT")?;
    ensure_column(conn, "teams", "piloto_1_id", "TEXT REFERENCES drivers(id)")?;
    ensure_column(conn, "teams", "piloto_2_id", "TEXT REFERENCES drivers(id)")?;
    ensure_column(conn, "teams", "facilities", "REAL NOT NULL DEFAULT 50.0")?;
    ensure_column(conn, "teams", "engineering", "REAL NOT NULL DEFAULT 50.0")?;
    ensure_column(conn, "teams", "morale", "REAL NOT NULL DEFAULT 1.0")?;
    ensure_column(conn, "teams", "aerodinamica", "REAL NOT NULL DEFAULT 50.0")?;
    ensure_column(conn, "teams", "motor", "REAL NOT NULL DEFAULT 50.0")?;
    ensure_column(conn, "teams", "chassi", "REAL NOT NULL DEFAULT 50.0")?;
    ensure_column(conn, "teams", "hierarquia_n1_id", "TEXT")?;
    ensure_column(conn, "teams", "hierarquia_n2_id", "TEXT")?;
    ensure_column(
        conn,
        "teams",
        "hierarquia_tensao",
        "REAL NOT NULL DEFAULT 0.0",
    )?;
    ensure_column(
        conn,
        "teams",
        "stats_vitorias",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(conn, "teams", "stats_podios", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "teams", "stats_poles", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "teams", "stats_pontos", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(
        conn,
        "teams",
        "stats_melhor_resultado",
        "INTEGER NOT NULL DEFAULT 99",
    )?;
    ensure_column(
        conn,
        "teams",
        "historico_vitorias",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "teams",
        "historico_podios",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "teams",
        "historico_poles",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "teams",
        "historico_pontos",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "teams",
        "historico_titulos_pilotos",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "teams",
        "temporada_atual",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(conn, "teams", "updated_at", "TEXT NOT NULL DEFAULT ''")?;

    conn.execute_batch(
        "
        UPDATE teams
        SET
            nome_curto = CASE
                WHEN nome_curto IS NULL OR TRIM(nome_curto) = '' THEN nome
                ELSE nome_curto
            END,
            cor_primaria = CASE
                WHEN cor_primaria IS NULL OR TRIM(cor_primaria) = '' THEN '#FFFFFF'
                ELSE cor_primaria
            END,
            cor_secundaria = CASE
                WHEN cor_secundaria IS NULL OR TRIM(cor_secundaria) = '' THEN '#000000'
                ELSE cor_secundaria
            END,
            pais_sede = CASE
                WHEN pais_sede IS NULL OR TRIM(pais_sede) = '' THEN 'Unknown'
                ELSE pais_sede
            END,
            ano_fundacao = CASE
                WHEN ano_fundacao IS NULL OR ano_fundacao <= 0 THEN CAST(strftime('%Y', 'now') AS INTEGER)
                ELSE ano_fundacao
            END,
            ativa = COALESCE(ativa, 1),
            hierarquia_status = CASE
                WHEN hierarquia_status IS NULL OR TRIM(hierarquia_status) = '' OR hierarquia_status = 'Independente' THEN 'estavel'
                ELSE LOWER(hierarquia_status)
            END,
            stats_vitorias = CASE
                WHEN stats_vitorias = 0 THEN COALESCE(temp_vitorias, 0)
                ELSE stats_vitorias
            END,
            stats_pontos = CASE
                WHEN stats_pontos = 0 THEN CAST(ROUND(COALESCE(temp_pontos, 0.0)) AS INTEGER)
                ELSE stats_pontos
            END,
            historico_vitorias = CASE
                WHEN historico_vitorias = 0 THEN COALESCE(carreira_vitorias, 0)
                ELSE historico_vitorias
            END,
            temporada_atual = CASE
                WHEN temporada_atual <= 0 THEN CAST(COALESCE((SELECT value FROM meta WHERE key = 'current_season'), '1') AS INTEGER)
                ELSE temporada_atual
            END,
            created_at = CASE
                WHEN created_at IS NULL OR TRIM(created_at) = '' THEN CURRENT_TIMESTAMP
                ELSE created_at
            END,
            updated_at = CASE
                WHEN updated_at IS NULL OR TRIM(updated_at) = '' THEN COALESCE(NULLIF(created_at, ''), CURRENT_TIMESTAMP)
                ELSE updated_at
            END;

        CREATE INDEX IF NOT EXISTS idx_teams_ativa ON teams(ativa);
        ",
    )?;

    Ok(())
}

fn migrate_v3(conn: &Connection) -> Result<(), DbError> {
    if !table_exists(conn, "contracts")? {
        return Ok(());
    }

    ensure_column(conn, "contracts", "piloto_nome", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(conn, "contracts", "equipe_nome", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(
        conn,
        "contracts",
        "duracao_anos",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(
        conn,
        "contracts",
        "salario_anual",
        "REAL NOT NULL DEFAULT 0.0",
    )?;
    ensure_column(conn, "contracts", "categoria", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(conn, "contracts", "created_at", "TEXT NOT NULL DEFAULT ''")?;

    conn.execute_batch(
        "
        UPDATE contracts
        SET
            piloto_nome = CASE
                WHEN piloto_nome IS NULL OR TRIM(piloto_nome) = '' THEN COALESCE(
                    (SELECT nome FROM drivers WHERE drivers.id = contracts.piloto_id),
                    piloto_id
                )
                ELSE piloto_nome
            END,
            equipe_nome = CASE
                WHEN equipe_nome IS NULL OR TRIM(equipe_nome) = '' THEN COALESCE(
                    (SELECT nome FROM teams WHERE teams.id = contracts.equipe_id),
                    equipe_id
                )
                ELSE equipe_nome
            END,
            duracao_anos = CASE
                WHEN MAX(
                    1,
                    CAST(temporada_fim AS INTEGER) - CAST(temporada_inicio AS INTEGER) + 1
                ) > COALESCE(duracao_anos, 0) THEN MAX(
                    1,
                    CAST(temporada_fim AS INTEGER) - CAST(temporada_inicio AS INTEGER) + 1
                )
                WHEN duracao_anos IS NULL OR duracao_anos <= 0 THEN 1
                ELSE duracao_anos
            END,
            salario_anual = CASE
                WHEN salario_anual IS NULL OR salario_anual <= 0 THEN COALESCE(salario, 0.0)
                ELSE salario_anual
            END,
            categoria = CASE
                WHEN categoria IS NULL OR TRIM(categoria) = '' THEN COALESCE(
                    (SELECT categoria FROM teams WHERE teams.id = contracts.equipe_id),
                    ''
                )
                ELSE categoria
            END,
            created_at = CASE
                WHEN created_at IS NULL OR TRIM(created_at) = '' THEN CURRENT_TIMESTAMP
                ELSE created_at
            END,
            papel = CASE
                WHEN papel IN ('Numero1', 'N1', 'Titular') THEN 'Numero1'
                WHEN papel IN ('Numero2', 'N2', 'Reserva', 'Junior') THEN 'Numero2'
                ELSE 'Numero2'
            END;

        CREATE INDEX IF NOT EXISTS idx_contracts_categoria ON contracts(categoria);
        ",
    )?;

    Ok(())
}

fn migrate_v4(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "seasons")? {
        ensure_column(
            conn,
            "seasons",
            "rodada_atual",
            "INTEGER NOT NULL DEFAULT 1",
        )?;
        ensure_column(conn, "seasons", "created_at", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(conn, "seasons", "updated_at", "TEXT NOT NULL DEFAULT ''")?;

        conn.execute_batch(
            "
            UPDATE seasons
            SET
                status = CASE
                    WHEN status IS NULL OR TRIM(status) = '' OR status = 'Ativa' THEN 'EmAndamento'
                    WHEN status = 'Finalizada' THEN 'Finalizada'
                    ELSE status
                END,
                rodada_atual = CASE
                    WHEN rodada_atual IS NULL OR rodada_atual <= 0 THEN 1
                    ELSE rodada_atual
                END,
                created_at = CASE
                    WHEN created_at IS NULL OR TRIM(created_at) = '' THEN CURRENT_TIMESTAMP
                    ELSE created_at
                END,
                updated_at = CASE
                    WHEN updated_at IS NULL OR TRIM(updated_at) = '' THEN COALESCE(NULLIF(created_at, ''), CURRENT_TIMESTAMP)
                    ELSE updated_at
                END;
            ",
        )?;
    }

    if table_exists(conn, "calendar")? {
        ensure_column(conn, "calendar", "season_id", "TEXT")?;
        ensure_column(conn, "calendar", "nome", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(conn, "calendar", "track_id", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "calendar", "track_name", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(conn, "calendar", "track_config", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(
            conn,
            "calendar",
            "temperatura",
            "REAL NOT NULL DEFAULT 25.0",
        )?;
        ensure_column(conn, "calendar", "voltas", "INTEGER NOT NULL DEFAULT 10")?;
        ensure_column(
            conn,
            "calendar",
            "duracao_corrida_min",
            "INTEGER NOT NULL DEFAULT 60",
        )?;
        ensure_column(
            conn,
            "calendar",
            "duracao_classificacao_min",
            "INTEGER NOT NULL DEFAULT 15",
        )?;
        ensure_column(
            conn,
            "calendar",
            "status",
            "TEXT NOT NULL DEFAULT 'Pendente'",
        )?;
        ensure_column(conn, "calendar", "horario", "TEXT NOT NULL DEFAULT '14:00'")?;

        conn.execute_batch(
            "
            UPDATE calendar
            SET
                season_id = CASE
                    WHEN season_id IS NULL OR TRIM(season_id) = '' THEN temporada_id
                    ELSE season_id
                END,
                nome = CASE
                    WHEN nome IS NULL OR TRIM(nome) = '' THEN ('Rodada ' || rodada || ' - ' || COALESCE(NULLIF(pista, ''), categoria))
                    ELSE nome
                END,
                track_name = CASE
                    WHEN track_name IS NULL OR TRIM(track_name) = '' THEN COALESCE(NULLIF(pista, ''), '')
                    ELSE track_name
                END,
                track_config = CASE
                    WHEN track_config IS NULL OR TRIM(track_config) = '' THEN ''
                    ELSE track_config
                END,
                clima = CASE
                    WHEN clima = 'Seco' THEN 'Dry'
                    WHEN clima = 'ChuvaLeve' THEN 'Damp'
                    WHEN clima = 'ChuvaForte' THEN 'HeavyRain'
                    WHEN clima = 'Nublado' THEN 'Damp'
                    ELSE COALESCE(NULLIF(clima, ''), 'Dry')
                END,
                duracao_corrida_min = CASE
                    WHEN duracao_corrida_min IS NULL OR duracao_corrida_min <= 0 THEN COALESCE(duracao, 60)
                    WHEN duracao_corrida_min = 60 AND COALESCE(duracao, 60) <> 60 THEN duracao
                    ELSE duracao_corrida_min
                END,
                status = CASE
                    WHEN status IS NULL OR TRIM(status) = '' THEN 'Pendente'
                    ELSE status
                END,
                horario = CASE
                    WHEN horario IS NULL OR TRIM(horario) = '' THEN '14:00'
                    ELSE horario
                END;

            CREATE INDEX IF NOT EXISTS idx_calendar_season_id ON calendar(season_id);
            CREATE INDEX IF NOT EXISTS idx_calendar_categoria ON calendar(categoria);
            ",
        )?;
    }

    Ok(())
}

fn migrate_v5(conn: &Connection) -> Result<(), DbError> {
    // The previous schema for race_results required a foreign key to `races(id)`
    // However, the application uses `calendar` entries as races and `races` table is entirely unused.
    // Since `race_results` was never populated prior to this update (no insert queries existed),
    // we can safely drop and recreate it to fix the foreign keys and add new module 25 columns.
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS race_results;
        CREATE TABLE race_results (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            race_id             TEXT NOT NULL,
            piloto_id           TEXT NOT NULL,
            equipe_id           TEXT NOT NULL,
            posicao_largada     INTEGER NOT NULL DEFAULT 0,
            posicao_final       INTEGER NOT NULL DEFAULT 0,
            voltas_completadas  INTEGER NOT NULL DEFAULT 0,
            dnf                 INTEGER NOT NULL DEFAULT 0,
            pontos              REAL NOT NULL DEFAULT 0.0,
            tempo_total         REAL NOT NULL DEFAULT 0.0,
            fastest_lap         INTEGER NOT NULL DEFAULT 0,
            dnf_reason          TEXT,
            dnf_segment         TEXT,
            incidents_count     INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (race_id)    REFERENCES calendar(id),
            FOREIGN KEY (piloto_id)  REFERENCES drivers(id),
            FOREIGN KEY (equipe_id)  REFERENCES teams(id)
        );
        CREATE INDEX IF NOT EXISTS idx_race_results_race ON race_results(race_id);
        CREATE INDEX IF NOT EXISTS idx_race_results_piloto ON race_results(piloto_id);
        "
    )?;

    Ok(())
}

fn migrate_v6(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS injuries;
        CREATE TABLE injuries (
            id                  TEXT PRIMARY KEY,
            pilot_id            TEXT NOT NULL,
            type                TEXT NOT NULL,
            modifier            REAL NOT NULL DEFAULT 1.0,
            races_total         INTEGER NOT NULL,
            races_remaining     INTEGER NOT NULL,
            skill_penalty       REAL NOT NULL DEFAULT 0.0,
            season              INTEGER NOT NULL,
            race_occurred       TEXT NOT NULL,
            active              INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (pilot_id) REFERENCES drivers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_injuries_pilot_id ON injuries(pilot_id);
        CREATE INDEX IF NOT EXISTS idx_injuries_active ON injuries(active);
        "
    )?;

    Ok(())
}

fn migrate_v7(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "teams")? {
        ensure_column(conn, "teams", "hierarquia_duelos_total", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "teams", "hierarquia_duelos_n2_vencidos", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "teams", "hierarquia_sequencia_n2", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "teams", "hierarquia_sequencia_n1", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "teams", "hierarquia_inversoes_temporada", "INTEGER NOT NULL DEFAULT 0")?;
    }
    Ok(())
}

fn migrate_v8(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "rivalries")? {
        // Adiciona os dois eixos de intensidade ao modelo dual
        ensure_column(conn, "rivalries", "historical_intensity", "REAL NOT NULL DEFAULT 0.0")?;
        ensure_column(conn, "rivalries", "recent_activity",      "REAL NOT NULL DEFAULT 0.0")?;
        // Temporada do último reforço — base para decisão de decaimento
        ensure_column(conn, "rivalries", "temporada_update",     "INTEGER NOT NULL DEFAULT 0")?;

        // Migra dados existentes: histórico recebe intensidade antiga; recente recebe 30% como calor residual
        conn.execute_batch(
            "UPDATE rivalries SET
                 historical_intensity = intensidade,
                 recent_activity      = ROUND(intensidade * 0.3, 2)
             WHERE historical_intensity = 0.0 AND intensidade > 0.0;"
        )?;
    }

    Ok(())
}

fn migrate_v9(conn: &Connection) -> Result<(), DbError> {
    // Guards necessários para testes de migração parcial que criam schemas sem todas as tabelas.
    if table_exists(conn, "contracts")? {
        // Tipo de contrato: Regular (padrão) ou Especial (sazonal de meio de ano)
        ensure_column(conn, "contracts", "tipo", "TEXT NOT NULL DEFAULT 'Regular'")?;
    }
    if table_exists(conn, "drivers")? {
        // Categoria especial ativa do piloto — separada da categoria regular de carreira
        ensure_column(conn, "drivers", "categoria_especial_ativa", "TEXT")?;
    }
    if table_exists(conn, "seasons")? {
        // Fase da temporada: BlocoRegular | JanelaConvocacao | BlocoEspecial
        ensure_column(conn, "seasons", "fase", "TEXT NOT NULL DEFAULT 'BlocoRegular'")?;
    }
    Ok(())
}

fn migrate_v10(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "contracts")? {
        // Classe do contrato especial (ex: "gt3", "mazda").
        // NULL em contratos regulares; preenchido em contratos especiais de categorias multi-classe.
        ensure_column(conn, "contracts", "classe", "TEXT")?;
    }
    Ok(())
}

fn migrate_v11(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "calendar")? {
        // Semana do ano (1-52) — unidade temporal interna do sistema.
        // Categorias regulares: semanas 2-40. Especiais: semanas 41-50.
        // Linhas existentes ficam com 0 (semana não atribuída — saves antigos).
        ensure_column(conn, "calendar", "week_of_year", "INTEGER NOT NULL DEFAULT 0")?;

        // Fase da temporada em que o evento ocorre.
        // BlocoRegular para categorias regulares; BlocoEspecial para especiais.
        ensure_column(conn, "calendar", "season_phase", "TEXT NOT NULL DEFAULT 'BlocoRegular'")?;
    }
    Ok(())
}

fn migrate_v12(conn: &Connection) -> Result<(), DbError> {
    if table_exists(conn, "calendar")? {
        // Papel narrativo fixo da corrida dentro da temporada (ThematicSlot).
        // NULL para saves pré-v12 — lidos como NaoClassificado pelo domain layer.
        // Sem DEFAULT: NULL é o estado semântico correto para saves legados.
        ensure_column(conn, "calendar", "thematic_slot", "TEXT")?;
    }
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> Result<(), DbError> {
    if !table_has_column(conn, table_name, column_name)? {
        conn.execute_batch(&format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            table_name, column_name, definition
        ))?;
    }

    Ok(())
}

fn table_has_column(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, DbError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get("name")?;
        if name == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, DbError> {
    let exists = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        rusqlite::params![table_name],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(exists > 0)
}

// ── DDL das 19 tabelas ────────────────────────────────────────────────────────

const MIGRATION_V1_DDL: &str = "

-- ── meta: configuração interna do banco ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ── config: configurações do usuário (espelha config.json) ───────────────────
CREATE TABLE IF NOT EXISTS config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ── drivers: pilotos (jogador + IA) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS drivers (
    id                       TEXT PRIMARY KEY,
    nome                     TEXT NOT NULL,
    is_jogador               INTEGER NOT NULL DEFAULT 0,
    idade                    INTEGER NOT NULL,
    nacionalidade            TEXT NOT NULL,
    genero                   TEXT NOT NULL DEFAULT 'M',
    categoria_atual          TEXT,
    status                   TEXT NOT NULL DEFAULT 'Ativo',
    personalidade_primaria   TEXT,
    personalidade_secundaria TEXT,
    ano_inicio_carreira      INTEGER NOT NULL DEFAULT 2024,

    -- 17 atributos — todos 0-100
    skill                    REAL NOT NULL DEFAULT 50.0,
    consistencia             REAL NOT NULL DEFAULT 50.0,
    racecraft                REAL NOT NULL DEFAULT 50.0,
    defesa                   REAL NOT NULL DEFAULT 50.0,
    ritmo_classificacao      REAL NOT NULL DEFAULT 50.0,
    gestao_pneus             REAL NOT NULL DEFAULT 50.0,
    habilidade_largada       REAL NOT NULL DEFAULT 50.0,
    adaptabilidade           REAL NOT NULL DEFAULT 50.0,
    fator_chuva              REAL NOT NULL DEFAULT 50.0,
    fitness                  REAL NOT NULL DEFAULT 50.0,
    experiencia              REAL NOT NULL DEFAULT 50.0,
    desenvolvimento          REAL NOT NULL DEFAULT 50.0,
    aggression               REAL NOT NULL DEFAULT 50.0,
    smoothness               REAL NOT NULL DEFAULT 50.0,
    midia                    REAL NOT NULL DEFAULT 50.0,
    mentalidade              REAL NOT NULL DEFAULT 50.0,
    confianca                REAL NOT NULL DEFAULT 50.0,

    -- Stats da temporada corrente
    temp_pontos              REAL NOT NULL DEFAULT 0.0,
    temp_vitorias            INTEGER NOT NULL DEFAULT 0,
    temp_podios              INTEGER NOT NULL DEFAULT 0,
    temp_poles               INTEGER NOT NULL DEFAULT 0,
    temp_corridas            INTEGER NOT NULL DEFAULT 0,
    temp_dnfs                INTEGER NOT NULL DEFAULT 0,
    temp_posicao_media       REAL NOT NULL DEFAULT 0.0,

    -- Stats de carreira acumuladas
    carreira_pontos_total    REAL NOT NULL DEFAULT 0.0,
    carreira_vitorias        INTEGER NOT NULL DEFAULT 0,
    carreira_podios          INTEGER NOT NULL DEFAULT 0,
    carreira_poles           INTEGER NOT NULL DEFAULT 0,
    carreira_corridas        INTEGER NOT NULL DEFAULT 0,
    carreira_temporadas      INTEGER NOT NULL DEFAULT 0,
    carreira_titulos         INTEGER NOT NULL DEFAULT 0,
    carreira_dnfs            INTEGER NOT NULL DEFAULT 0,

    -- Tracking dinâmico
    motivacao                REAL NOT NULL DEFAULT 75.0,
    historico_circuitos      TEXT NOT NULL DEFAULT '{}',
    ultimos_resultados       TEXT NOT NULL DEFAULT '[]',
    melhor_resultado_temp    INTEGER,
    temporadas_na_categoria  INTEGER NOT NULL DEFAULT 0,
    corridas_na_categoria    INTEGER NOT NULL DEFAULT 0,
    temporadas_motivacao_baixa INTEGER NOT NULL DEFAULT 0
);

-- ── teams: equipes ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS teams (
    id                   TEXT PRIMARY KEY,
    nome                 TEXT NOT NULL,
    categoria            TEXT NOT NULL,
    is_player_team       INTEGER NOT NULL DEFAULT 0,

    -- Performance (Módulo 16)
    car_performance      REAL NOT NULL DEFAULT 50.0,
    reliability          REAL NOT NULL DEFAULT 50.0,
    budget               REAL NOT NULL DEFAULT 1000000.0,
    prestige             REAL NOT NULL DEFAULT 50.0,

    -- Hierarquia (Módulo 17)
    hierarquia_status    TEXT NOT NULL DEFAULT 'Independente',
    parent_team_id       TEXT,
    aceita_rookies       INTEGER NOT NULL DEFAULT 1,
    meta_posicao         INTEGER NOT NULL DEFAULT 10,

    -- Stats da temporada corrente (Módulo 18)
    temp_pontos          REAL NOT NULL DEFAULT 0.0,
    temp_posicao         INTEGER NOT NULL DEFAULT 0,
    temp_vitorias        INTEGER NOT NULL DEFAULT 0,

    -- Stats de carreira
    carreira_titulos     INTEGER NOT NULL DEFAULT 0,
    carreira_vitorias    INTEGER NOT NULL DEFAULT 0,

    created_at           TEXT NOT NULL DEFAULT ''
);

-- ── contracts: contratos piloto↔equipe ───────────────────────────────────────
CREATE TABLE IF NOT EXISTS contracts (
    id                TEXT PRIMARY KEY,
    piloto_id         TEXT NOT NULL,
    equipe_id         TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'Ativo',
    papel             TEXT NOT NULL DEFAULT 'Titular',
    salario           REAL NOT NULL DEFAULT 0.0,
    temporada_inicio  TEXT NOT NULL,
    temporada_fim     TEXT NOT NULL,
    clausulas         TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (piloto_id) REFERENCES drivers(id),
    FOREIGN KEY (equipe_id) REFERENCES teams(id)
);

-- ── seasons: temporadas ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS seasons (
    id      TEXT PRIMARY KEY,
    numero  INTEGER NOT NULL,
    ano     INTEGER NOT NULL,
    status  TEXT NOT NULL DEFAULT 'Ativa'
);

-- ── calendar: calendário de corridas ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS calendar (
    id           TEXT PRIMARY KEY,
    temporada_id TEXT NOT NULL,
    rodada       INTEGER NOT NULL,
    pista        TEXT NOT NULL,
    categoria    TEXT NOT NULL,
    clima        TEXT NOT NULL DEFAULT 'Seco',
    duracao      INTEGER NOT NULL DEFAULT 60,
    data         TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (temporada_id) REFERENCES seasons(id)
);

-- ── races: corridas disputadas ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS races (
    id           TEXT PRIMARY KEY,
    temporada_id TEXT NOT NULL,
    calendar_id  TEXT NOT NULL,
    rodada       INTEGER NOT NULL,
    pista        TEXT NOT NULL,
    data         TEXT NOT NULL DEFAULT '',
    clima        TEXT NOT NULL DEFAULT 'Seco',
    status       TEXT NOT NULL DEFAULT 'Pendente',
    FOREIGN KEY (temporada_id) REFERENCES seasons(id),
    FOREIGN KEY (calendar_id)  REFERENCES calendar(id)
);

-- ── race_results: resultados por piloto por corrida ───────────────────────────
CREATE TABLE IF NOT EXISTS race_results (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    race_id             TEXT NOT NULL,
    piloto_id           TEXT NOT NULL,
    equipe_id           TEXT NOT NULL,
    posicao_largada     INTEGER NOT NULL DEFAULT 0,
    posicao_final       INTEGER NOT NULL DEFAULT 0,
    voltas_completadas  INTEGER NOT NULL DEFAULT 0,
    dnf                 INTEGER NOT NULL DEFAULT 0,
    pontos              REAL NOT NULL DEFAULT 0.0,
    tempo_total         REAL NOT NULL DEFAULT 0.0,
    fastest_lap         INTEGER NOT NULL DEFAULT 0,
    dnf_reason          TEXT,
    dnf_segment         TEXT,
    incidents_count     INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (race_id)    REFERENCES races(id),
    FOREIGN KEY (piloto_id)  REFERENCES drivers(id),
    FOREIGN KEY (equipe_id)  REFERENCES teams(id)
);

-- ── standings: classificação por temporada ────────────────────────────────────
CREATE TABLE IF NOT EXISTS standings (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id TEXT NOT NULL,
    piloto_id    TEXT NOT NULL,
    equipe_id    TEXT NOT NULL,
    categoria    TEXT NOT NULL,
    posicao      INTEGER NOT NULL DEFAULT 0,
    pontos       REAL NOT NULL DEFAULT 0.0,
    vitorias     INTEGER NOT NULL DEFAULT 0,
    podios       INTEGER NOT NULL DEFAULT 0,
    poles        INTEGER NOT NULL DEFAULT 0,
    corridas     INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (temporada_id) REFERENCES seasons(id),
    FOREIGN KEY (piloto_id)    REFERENCES drivers(id),
    FOREIGN KEY (equipe_id)    REFERENCES teams(id)
);

-- ── licenses: licenças por piloto ────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS licenses (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT,
    piloto_id                TEXT NOT NULL,
    nivel                    TEXT NOT NULL,
    categoria_origem         TEXT NOT NULL,
    data_obtencao            TEXT NOT NULL DEFAULT '',
    temporadas_na_categoria  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (piloto_id) REFERENCES drivers(id)
);

-- ── injuries: lesões ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS injuries (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    piloto_id         TEXT NOT NULL,
    tipo              TEXT NOT NULL DEFAULT 'Leve',
    corridas_restantes INTEGER NOT NULL DEFAULT 0,
    temporada_id      TEXT NOT NULL,
    descricao         TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (piloto_id)    REFERENCES drivers(id),
    FOREIGN KEY (temporada_id) REFERENCES seasons(id)
);

-- ── market: estado geral do mercado de transferências ────────────────────────
CREATE TABLE IF NOT EXISTS market (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'Fechado',
    fase         TEXT NOT NULL DEFAULT 'PreTemporada',
    inicio       TEXT NOT NULL DEFAULT '',
    fim          TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (temporada_id) REFERENCES seasons(id)
);

-- ── market_proposals: propostas equipe→piloto ─────────────────────────────────
CREATE TABLE IF NOT EXISTS market_proposals (
    id              TEXT PRIMARY KEY,
    temporada_id    TEXT NOT NULL,
    equipe_id       TEXT NOT NULL,
    piloto_id       TEXT NOT NULL,
    papel           TEXT NOT NULL DEFAULT 'Titular',
    salario         REAL NOT NULL DEFAULT 0.0,
    status          TEXT NOT NULL DEFAULT 'Pendente',
    motivo_recusa   TEXT,
    criado_em       TEXT NOT NULL DEFAULT '',
    respondido_em   TEXT,
    FOREIGN KEY (temporada_id) REFERENCES seasons(id),
    FOREIGN KEY (equipe_id)    REFERENCES teams(id),
    FOREIGN KEY (piloto_id)    REFERENCES drivers(id)
);

-- ── news: notícias do simulador ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS news (
    id           TEXT PRIMARY KEY,
    tipo         TEXT NOT NULL,
    titulo       TEXT NOT NULL,
    texto        TEXT NOT NULL,
    chave_dedup  TEXT UNIQUE,
    temporada_id TEXT NOT NULL,
    rodada       INTEGER NOT NULL DEFAULT 0,
    criado_em    TEXT NOT NULL DEFAULT '',
    lida         INTEGER NOT NULL DEFAULT 0
);

-- ── rivalries: rivalidades entre pilotos ─────────────────────────────────────
CREATE TABLE IF NOT EXISTS rivalries (
    id                 TEXT PRIMARY KEY,
    piloto1_id         TEXT NOT NULL,
    piloto2_id         TEXT NOT NULL,
    intensidade        REAL NOT NULL DEFAULT 0.0,
    tipo               TEXT NOT NULL DEFAULT 'Normal',
    criado_em          TEXT NOT NULL DEFAULT '',
    ultima_atualizacao TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (piloto1_id) REFERENCES drivers(id),
    FOREIGN KEY (piloto2_id) REFERENCES drivers(id)
);

-- ── retired: pilotos aposentados (snapshot histórico) ────────────────────────
CREATE TABLE IF NOT EXISTS retired (
    piloto_id                TEXT PRIMARY KEY,
    nome                     TEXT NOT NULL,
    temporada_aposentadoria  TEXT NOT NULL,
    categoria_final          TEXT NOT NULL,
    estatisticas             TEXT NOT NULL DEFAULT '{}',
    motivo                   TEXT NOT NULL DEFAULT 'Aposentadoria'
);

-- ── history_seasons: resultado final de cada temporada ───────────────────────
CREATE TABLE IF NOT EXISTS history_seasons (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id        TEXT NOT NULL,
    ano                 INTEGER NOT NULL,
    categoria           TEXT NOT NULL,
    campeao_piloto_id   TEXT NOT NULL,
    campeao_equipe_id   TEXT NOT NULL,
    classificacao_final TEXT NOT NULL DEFAULT '[]'
);

-- ── history_general: dados históricos genéricos (chave/valor) ────────────────
CREATE TABLE IF NOT EXISTS history_general (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT ''
);

-- ── Índices nas colunas mais consultadas ──────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_drivers_status    ON drivers(status);
CREATE INDEX IF NOT EXISTS idx_drivers_categoria ON drivers(categoria_atual);
CREATE INDEX IF NOT EXISTS idx_teams_categoria   ON teams(categoria);
CREATE INDEX IF NOT EXISTS idx_contracts_piloto  ON contracts(piloto_id);
CREATE INDEX IF NOT EXISTS idx_contracts_equipe  ON contracts(equipe_id);
CREATE INDEX IF NOT EXISTS idx_contracts_status  ON contracts(status);
CREATE INDEX IF NOT EXISTS idx_calendar_temporada ON calendar(temporada_id);
CREATE INDEX IF NOT EXISTS idx_races_temporada   ON races(temporada_id);
CREATE INDEX IF NOT EXISTS idx_race_results_race ON race_results(race_id);
CREATE INDEX IF NOT EXISTS idx_race_results_piloto ON race_results(piloto_id);
CREATE INDEX IF NOT EXISTS idx_standings_temporada ON standings(temporada_id);
CREATE INDEX IF NOT EXISTS idx_standings_piloto  ON standings(piloto_id);
CREATE INDEX IF NOT EXISTS idx_news_temporada    ON news(temporada_id);
CREATE INDEX IF NOT EXISTS idx_news_lida         ON news(lida);
CREATE INDEX IF NOT EXISTS idx_injuries_piloto   ON injuries(piloto_id);
CREATE INDEX IF NOT EXISTS idx_market_proposals_piloto ON market_proposals(piloto_id);
CREATE INDEX IF NOT EXISTS idx_market_proposals_equipe ON market_proposals(equipe_id);
CREATE INDEX IF NOT EXISTS idx_rivalries_piloto1      ON rivalries(piloto1_id);
CREATE INDEX IF NOT EXISTS idx_rivalries_piloto2      ON rivalries(piloto2_id);

";

// ── Seed inicial da tabela meta ───────────────────────────────────────────────

fn seed_meta(conn: &Connection) -> Result<(), DbError> {
    let seeds = [
        ("next_driver_id", "1"),
        ("next_team_id", "1"),
        ("next_season_id", "1"),
        ("next_race_id", "1"),
        ("next_contract_id", "1"),
        ("next_news_id", "1"),
        ("next_rivalry_id", "1"),
        ("current_season", "1"),
        ("current_year", "2024"),
        ("difficulty", "Normal"),
    ];

    for (key, value) in &seeds {
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_pending_migrates_teams_to_v2_and_preserves_existing_data() {
        let conn = Connection::open_in_memory().expect("in-memory db");

        conn.execute_batch(
            "
            CREATE TABLE meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO meta (key, value) VALUES ('schema_version', '1');

            CREATE TABLE teams (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL,
                categoria TEXT NOT NULL,
                is_player_team INTEGER NOT NULL DEFAULT 0,
                car_performance REAL NOT NULL DEFAULT 50.0,
                reliability REAL NOT NULL DEFAULT 50.0,
                budget REAL NOT NULL DEFAULT 1000000.0,
                prestige REAL NOT NULL DEFAULT 50.0,
                hierarquia_status TEXT NOT NULL DEFAULT 'Independente',
                parent_team_id TEXT,
                aceita_rookies INTEGER NOT NULL DEFAULT 1,
                meta_posicao INTEGER NOT NULL DEFAULT 10,
                temp_pontos REAL NOT NULL DEFAULT 0.0,
                temp_posicao INTEGER NOT NULL DEFAULT 0,
                temp_vitorias INTEGER NOT NULL DEFAULT 0,
                carreira_titulos INTEGER NOT NULL DEFAULT 0,
                carreira_vitorias INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT ''
            );

            INSERT INTO teams (
                id, nome, categoria, temp_pontos, temp_vitorias, carreira_vitorias, created_at
            ) VALUES (
                'T001', 'Equipe Legada', 'gt3', 42.0, 3, 9, '2026-01-01T12:00:00'
            );
            ",
        )
        .expect("legacy schema should be created");

        run_pending(&conn).expect("migration should succeed");

        assert_eq!(get_schema_version(&conn).expect("schema version"), 11);
        assert!(column_exists(&conn, "teams", "nome_curto"));
        assert!(column_exists(&conn, "teams", "stats_vitorias"));
        assert!(column_exists(&conn, "teams", "stats_pontos"));
        assert!(column_exists(&conn, "teams", "historico_vitorias"));
        assert!(column_exists(&conn, "teams", "updated_at"));

        let row: (String, i64, i64, i64) = conn
            .query_row(
                "SELECT nome_curto, stats_vitorias, stats_pontos, historico_vitorias
                 FROM teams WHERE id = 'T001'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated row");

        assert_eq!(row.0, "Equipe Legada");
        assert_eq!(row.1, 3);
        assert_eq!(row.2, 42);
        assert_eq!(row.3, 9);

        let idx_ativa_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_teams_ativa'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .expect("index query");
        assert!(idx_ativa_exists);
    }

    #[test]
    fn test_run_pending_migrates_contracts_to_v3_and_backfills_new_fields() {
        let conn = Connection::open_in_memory().expect("in-memory db");

        conn.execute_batch(
            "
            CREATE TABLE meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO meta (key, value) VALUES ('schema_version', '2');

            CREATE TABLE drivers (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL
            );

            CREATE TABLE teams (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL,
                categoria TEXT NOT NULL
            );

            CREATE TABLE contracts (
                id TEXT PRIMARY KEY,
                piloto_id TEXT NOT NULL,
                equipe_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Ativo',
                papel TEXT NOT NULL DEFAULT 'Titular',
                salario REAL NOT NULL DEFAULT 0.0,
                temporada_inicio TEXT NOT NULL,
                temporada_fim TEXT NOT NULL,
                clausulas TEXT NOT NULL DEFAULT '{}'
            );

            INSERT INTO drivers (id, nome) VALUES ('P001', 'Piloto Legado');
            INSERT INTO teams (id, nome, categoria) VALUES ('T001', 'Equipe Legada', 'gt3');
            INSERT INTO contracts (
                id, piloto_id, equipe_id, status, papel, salario, temporada_inicio, temporada_fim
            ) VALUES (
                'C001', 'P001', 'T001', 'Ativo', 'Titular', 150000.0, '1', '3'
            );
            ",
        )
        .expect("legacy schema should be created");

        run_pending(&conn).expect("migration should succeed");

        assert_eq!(get_schema_version(&conn).expect("schema version"), 11);
        assert!(column_exists(&conn, "contracts", "piloto_nome"));
        assert!(column_exists(&conn, "contracts", "equipe_nome"));
        assert!(column_exists(&conn, "contracts", "duracao_anos"));
        assert!(column_exists(&conn, "contracts", "salario_anual"));
        assert!(column_exists(&conn, "contracts", "categoria"));
        assert!(column_exists(&conn, "contracts", "created_at"));

        let row: (String, String, i64, f64, String, String) = conn
            .query_row(
                "SELECT piloto_nome, equipe_nome, duracao_anos, salario_anual, categoria, papel
                 FROM contracts WHERE id = 'C001'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("migrated row");

        assert_eq!(row.0, "Piloto Legado");
        assert_eq!(row.1, "Equipe Legada");
        assert_eq!(row.2, 3);
        assert_eq!(row.3, 150000.0);
        assert_eq!(row.4, "gt3");
        assert_eq!(row.5, "Numero1");
    }

    #[test]
    fn test_run_pending_migrates_seasons_and_calendar_to_v4() {
        let conn = Connection::open_in_memory().expect("in-memory db");

        conn.execute_batch(
            "
            CREATE TABLE meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO meta (key, value) VALUES ('schema_version', '3');

            CREATE TABLE seasons (
                id TEXT PRIMARY KEY,
                numero INTEGER NOT NULL,
                ano INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Ativa'
            );

            CREATE TABLE calendar (
                id TEXT PRIMARY KEY,
                temporada_id TEXT NOT NULL,
                rodada INTEGER NOT NULL,
                pista TEXT NOT NULL,
                categoria TEXT NOT NULL,
                clima TEXT NOT NULL DEFAULT 'Seco',
                duracao INTEGER NOT NULL DEFAULT 60,
                data TEXT NOT NULL DEFAULT ''
            );

            INSERT INTO seasons (id, numero, ano, status) VALUES ('S001', 1, 2024, 'Ativa');
            INSERT INTO calendar (id, temporada_id, rodada, pista, categoria, clima, duracao)
            VALUES ('R001', 'S001', 1, 'Laguna Seca', 'mazda_rookie', 'Seco', 15);
            ",
        )
        .expect("legacy schema should be created");

        run_pending(&conn).expect("migration should succeed");

        assert_eq!(get_schema_version(&conn).expect("schema version"), 11);
        assert!(column_exists(&conn, "seasons", "rodada_atual"));
        assert!(column_exists(&conn, "seasons", "updated_at"));
        assert!(column_exists(&conn, "calendar", "season_id"));
        assert!(column_exists(&conn, "calendar", "track_name"));
        assert!(column_exists(&conn, "calendar", "duracao_corrida_min"));
        assert!(column_exists(&conn, "calendar", "status"));

        let season_row: (String, i64) = conn
            .query_row(
                "SELECT status, rodada_atual FROM seasons WHERE id = 'S001'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("season row");
        assert_eq!(season_row.0, "EmAndamento");
        assert_eq!(season_row.1, 1);

        let calendar_row: (String, String, i64, String) = conn
            .query_row(
                "SELECT season_id, track_name, duracao_corrida_min, clima FROM calendar WHERE id = 'R001'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("calendar row");
        assert_eq!(calendar_row.0, "S001");
        assert_eq!(calendar_row.1, "Laguna Seca");
        assert_eq!(calendar_row.2, 15);
        assert_eq!(calendar_row.3, "Dry");
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .expect("pragma table_info");
        let mut rows = stmt.query([]).expect("query pragma");

        while let Some(row) = rows.next().expect("next row") {
            let name: String = row.get("name").expect("column name");
            if name == column {
                return true;
            }
        }

        false
    }
}
