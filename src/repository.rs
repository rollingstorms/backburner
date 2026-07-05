use anyhow::{Context as _, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{
    Context, Source, Task, TaskCommand, TaskDetails, TaskFileRef, TaskNote, TaskStatus, now_string,
    parse_timestamp, today_key,
};

pub struct Repository {
    conn: Connection,
}

pub struct CreateTask {
    pub title: String,
    pub status: TaskStatus,
    pub planned_date_key: Option<String>,
    pub source: Source,
    pub notes: Vec<String>,
    pub files: Vec<FileRefInput>,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileRefInput {
    pub path: String,
    pub line: Option<i64>,
}

impl Repository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn create(&mut self, input: CreateTask) -> Result<TaskDetails> {
        let title = input.title.trim();
        if title.is_empty() {
            bail!("title cannot be empty");
        }
        let now = now_string();
        let order = self.next_sort_order(input.status)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            r#"
            insert into tasks
              (title, status, planned_date_key, source, created_at, updated_at, sort_order, metadata_json)
            values
              (?1, ?2, ?3, ?4, ?5, ?5, ?6, '{}')
            "#,
            params![
                title,
                input.status.as_str(),
                input.planned_date_key,
                input.source.as_str(),
                now,
                order
            ],
        )?;
        let id = tx.last_insert_rowid();
        for note in input.notes {
            let trimmed = note.trim();
            if !trimmed.is_empty() {
                tx.execute(
                    "insert into task_notes (task_id, body, source, created_at) values (?1, ?2, ?3, ?4)",
                    params![id, trimmed, input.source.as_str(), now],
                )?;
            }
        }
        for file in input.files {
            tx.execute(
                "insert into task_file_refs (task_id, path, line) values (?1, ?2, ?3)",
                params![id, file.path, file.line],
            )?;
        }
        for command in input.commands {
            let trimmed = command.trim();
            if !trimmed.is_empty() {
                tx.execute(
                    "insert into task_commands (task_id, command, created_at) values (?1, ?2, ?3)",
                    params![id, trimmed, now],
                )?;
            }
        }
        tx.commit()?;
        self.details(id)
    }

    pub fn list(&self, status: TaskStatus) -> Result<Vec<TaskDetails>> {
        let mut stmt = self.conn.prepare(
            r#"
            select id, title, status, planned_date_key, source, created_at, updated_at,
                   completed_at, archived_at, sort_order, metadata_json
            from tasks
            where status = ?1
            order by planned_date_key is null, planned_date_key asc, sort_order asc, created_at asc
            "#,
        )?;
        let tasks = stmt
            .query_map(params![status.as_str()], map_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        tasks
            .into_iter()
            .map(|task| self.details_for_task(task))
            .collect()
    }

    pub fn details(&self, id: i64) -> Result<TaskDetails> {
        let task = self
            .find_task(id)?
            .with_context(|| format!("task #{id} not found"))?;
        self.details_for_task(task)
    }

    pub fn set_completed(&self, id: i64, completed: bool) -> Result<()> {
        self.ensure_exists(id)?;
        let now = now_string();
        let completed_at = if completed { Some(now.as_str()) } else { None };
        self.conn.execute(
            "update tasks set completed_at = ?1, updated_at = ?2 where id = ?3",
            params![completed_at, now, id],
        )?;
        Ok(())
    }

    pub fn move_to(&self, id: i64, status: TaskStatus) -> Result<()> {
        let task = self
            .find_task(id)?
            .with_context(|| format!("task #{id} not found"))?;
        if task.status == status {
            return Ok(());
        }
        let now = now_string();
        let completed_at = match status {
            TaskStatus::Archived => task
                .completed_at
                .as_deref()
                .unwrap_or(&now)
                .to_string()
                .into(),
            TaskStatus::Today | TaskStatus::Backburner => None,
        };
        let archived_at = if status == TaskStatus::Archived {
            Some(now.as_str())
        } else {
            None
        };
        self.conn.execute(
            r#"
            update tasks
            set status = ?1, updated_at = ?2, completed_at = ?3, archived_at = ?4, sort_order = ?5
            where id = ?6
            "#,
            params![
                status.as_str(),
                now,
                completed_at,
                archived_at,
                self.next_sort_order(status)?,
                id
            ],
        )?;
        Ok(())
    }

    pub fn plan(&self, id: i64, planned_date_key: Option<String>) -> Result<()> {
        self.ensure_exists(id)?;
        let now = now_string();
        self.conn.execute(
            "update tasks set planned_date_key = ?1, updated_at = ?2 where id = ?3",
            params![planned_date_key, now, id],
        )?;
        Ok(())
    }

    pub fn add_note(&self, id: i64, body: &str, source: Source) -> Result<()> {
        self.ensure_exists(id)?;
        let body = body.trim();
        if body.is_empty() {
            bail!("note cannot be empty");
        }
        self.conn.execute(
            "insert into task_notes (task_id, body, source, created_at) values (?1, ?2, ?3, ?4)",
            params![id, body, source.as_str(), now_string()],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.ensure_exists(id)?;
        self.conn
            .execute("delete from tasks where id = ?1", params![id])?;
        Ok(())
    }

    pub fn finish_session(&mut self) -> Result<FinishSessionResult> {
        let today = self.list_tasks_only(TaskStatus::Today)?;
        if today.is_empty() {
            return Ok(FinishSessionResult::default());
        }
        let mut archived = 0;
        let mut backburnered = 0;
        let mut next_backburner = self.next_sort_order(TaskStatus::Backburner)?;
        let mut next_archive = self.next_sort_order(TaskStatus::Archived)?;
        let now = now_string();
        let tx = self.conn.transaction()?;
        for task in today {
            if task.completed_at.is_some() {
                tx.execute(
                    r#"
                    update tasks
                    set status = 'archived', updated_at = ?1, archived_at = ?1, sort_order = ?2
                    where id = ?3
                    "#,
                    params![now, next_archive, task.id],
                )?;
                next_archive += 1;
                archived += 1;
            } else {
                tx.execute(
                    r#"
                    update tasks
                    set status = 'backburner', updated_at = ?1, completed_at = null,
                        archived_at = null, sort_order = ?2
                    where id = ?3
                    "#,
                    params![now, next_backburner, task.id],
                )?;
                next_backburner += 1;
                backburnered += 1;
            }
        }
        tx.commit()?;
        Ok(FinishSessionResult {
            archived,
            backburnered,
        })
    }

    pub fn promote_due(&mut self) -> Result<usize> {
        let key = today_key();
        let due = {
            let mut stmt = self.conn.prepare(
                r#"
                select id, title, status, planned_date_key, source, created_at, updated_at,
                       completed_at, archived_at, sort_order, metadata_json
                from tasks
                where status = 'backburner'
                  and planned_date_key is not null
                  and planned_date_key <= ?1
                order by planned_date_key asc, sort_order asc, created_at asc
                "#,
            )?;
            stmt.query_map(params![key], map_task)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        if due.is_empty() {
            return Ok(0);
        }
        let mut next_today = self.next_sort_order(TaskStatus::Today)?;
        let now = now_string();
        let tx = self.conn.transaction()?;
        for task in &due {
            tx.execute(
                r#"
                update tasks
                set status = 'today', planned_date_key = null, updated_at = ?1, completed_at = null,
                    archived_at = null, sort_order = ?2
                where id = ?3
                "#,
                params![now, next_today, task.id],
            )?;
            next_today += 1;
        }
        tx.commit()?;
        Ok(due.len())
    }

    pub fn context(&mut self) -> Result<Context> {
        let promoted = self.promote_due()?;
        Ok(Context {
            today: self.list(TaskStatus::Today)?,
            backburner: self.list(TaskStatus::Backburner)?,
            promoted,
        })
    }

    fn details_for_task(&self, task: Task) -> Result<TaskDetails> {
        let notes = self.notes(task.id)?;
        let files = self.files(task.id)?;
        let commands = self.commands(task.id)?;
        Ok(TaskDetails {
            task,
            notes,
            files,
            commands,
        })
    }

    fn find_task(&self, id: i64) -> Result<Option<Task>> {
        self.conn
            .query_row(
                r#"
                select id, title, status, planned_date_key, source, created_at, updated_at,
                       completed_at, archived_at, sort_order, metadata_json
                from tasks
                where id = ?1
                "#,
                params![id],
                map_task,
            )
            .optional()
            .map_err(Into::into)
    }

    fn list_tasks_only(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            r#"
            select id, title, status, planned_date_key, source, created_at, updated_at,
                   completed_at, archived_at, sort_order, metadata_json
            from tasks
            where status = ?1
            order by planned_date_key is null, planned_date_key asc, sort_order asc, created_at asc
            "#,
        )?;
        Ok(stmt
            .query_map(params![status.as_str()], map_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn notes(&self, task_id: i64) -> Result<Vec<TaskNote>> {
        let mut stmt = self.conn.prepare(
            "select id, task_id, body, source, created_at from task_notes where task_id = ?1 order by id",
        )?;
        Ok(stmt
            .query_map(params![task_id], |row| {
                Ok(TaskNote {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    body: row.get(2)?,
                    source: row.get::<_, String>(3)?.parse().map_err(to_sql_err)?,
                    created_at: parse_timestamp(row.get(4)?),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn files(&self, task_id: i64) -> Result<Vec<TaskFileRef>> {
        let mut stmt = self.conn.prepare(
            "select id, task_id, path, line, label from task_file_refs where task_id = ?1 order by id",
        )?;
        Ok(stmt
            .query_map(params![task_id], |row| {
                Ok(TaskFileRef {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    path: row.get(2)?,
                    line: row.get(3)?,
                    label: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn commands(&self, task_id: i64) -> Result<Vec<TaskCommand>> {
        let mut stmt = self.conn.prepare(
            "select id, task_id, command, result_summary, created_at from task_commands where task_id = ?1 order by id",
        )?;
        Ok(stmt
            .query_map(params![task_id], |row| {
                Ok(TaskCommand {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    command: row.get(2)?,
                    result_summary: row.get(3)?,
                    created_at: parse_timestamp(row.get(4)?),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn ensure_exists(&self, id: i64) -> Result<()> {
        if self.find_task(id)?.is_none() {
            bail!("task #{id} not found");
        }
        Ok(())
    }

    fn next_sort_order(&self, status: TaskStatus) -> Result<i64> {
        Ok(self.conn.query_row(
            "select coalesce(max(sort_order), -1) + 1 from tasks where status = ?1",
            params![status.as_str()],
            |row| row.get(0),
        )?)
    }
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishSessionResult {
    pub archived: usize,
    pub backburnered: usize,
}

pub fn parse_file_ref(value: &str) -> FileRefInput {
    let Some((path, line)) = value.rsplit_once(':') else {
        return FileRefInput {
            path: value.to_string(),
            line: None,
        };
    };
    match line.parse::<i64>() {
        Ok(line) if line > 0 => FileRefInput {
            path: path.to_string(),
            line: Some(line),
        },
        _ => FileRefInput {
            path: value.to_string(),
            line: None,
        },
    }
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get::<_, String>(2)?.parse().map_err(to_sql_err)?,
        planned_date_key: row.get(3)?,
        source: row.get::<_, String>(4)?.parse().map_err(to_sql_err)?,
        created_at: parse_timestamp(row.get(5)?),
        updated_at: parse_timestamp(row.get(6)?),
        completed_at: row.get(7)?,
        archived_at: row.get(8)?,
        sort_order: row.get(9)?,
        metadata_json: row.get(10)?,
    })
}

fn to_sql_err(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}
