use rusqlite::{Connection, OptionalExtension};

/// Lê um valor da tabela meta. Retorna None se a chave não existir.
pub fn get_meta_value(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .optional()
}

/// Atualiza um valor da tabela meta.
///
/// Presume que a chave já existe (inserida pelas migrations).
/// Não faz INSERT, não faz UPSERT, não faz INSERT OR REPLACE.
/// Se a chave não existir, a operação é silenciosa — comportamento idêntico
/// ao UPDATE inline que substitui.
pub fn set_meta_value(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = ?2",
        rusqlite::params![value, key],
    )?;
    Ok(())
}

/// Atalho semântico: atualiza current_season.
pub fn set_current_season(conn: &Connection, numero: i32) -> Result<(), rusqlite::Error> {
    set_meta_value(conn, "current_season", &numero.to_string())
}

/// Atalho semântico: atualiza current_year.
pub fn set_current_year(conn: &Connection, year: i32) -> Result<(), rusqlite::Error> {
    set_meta_value(conn, "current_year", &year.to_string())
}
