use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(
        r#"
        create table if not exists schema_migrations (
          version integer primary key
        );

        create table if not exists tasks (
          id integer primary key,
          title text not null,
          status text not null check (status in ('today', 'backburner', 'archived')),
          planned_date_key text null,
          source text not null check (source in ('human', 'agent')),
          created_at text not null,
          updated_at text not null,
          completed_at text null,
          archived_at text null,
          sort_order integer not null,
          session_key text null,
          metadata_json text not null default '{}'
        );

        create table if not exists task_notes (
          id integer primary key,
          task_id integer not null references tasks(id) on delete cascade,
          body text not null,
          source text not null check (source in ('human', 'agent')),
          created_at text not null
        );

        create table if not exists task_file_refs (
          id integer primary key,
          task_id integer not null references tasks(id) on delete cascade,
          path text not null,
          line integer null,
          label text null
        );

        create table if not exists task_commands (
          id integer primary key,
          task_id integer not null references tasks(id) on delete cascade,
          command text not null,
          result_summary text null,
          created_at text not null
        );

        insert or ignore into schema_migrations(version) values (1);
        "#,
    )?;
    add_column_if_missing(conn, "tasks", "session_key", "text null")?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("pragma table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == column);
    if !exists {
        conn.execute_batch(&format!(
            "alter table {table} add column {column} {definition}"
        ))?;
    }
    Ok(())
}
