use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::{Result, bail};
use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Today,
    Backburner,
    Archived,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Today => "today",
            TaskStatus::Backburner => "backburner",
            TaskStatus::Archived => "archived",
        }
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "today" => Ok(TaskStatus::Today),
            "backburner" => Ok(TaskStatus::Backburner),
            "archived" | "archive" => Ok(TaskStatus::Archived),
            _ => bail!("unknown status '{value}'"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Human,
    Agent,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Human => "human",
            Source::Agent => "agent",
        }
    }
}

impl FromStr for Source {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "human" => Ok(Source::Human),
            "agent" => Ok(Source::Agent),
            _ => bail!("unknown source '{value}'"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub status: TaskStatus,
    pub planned_date_key: Option<String>,
    pub source: Source,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
    pub sort_order: i64,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNote {
    pub id: i64,
    pub task_id: i64,
    pub body: String,
    pub source: Source,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFileRef {
    pub id: i64,
    pub task_id: i64,
    pub path: String,
    pub line: Option<i64>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCommand {
    pub id: i64,
    pub task_id: i64,
    pub command: String,
    pub result_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetails {
    pub task: Task,
    pub notes: Vec<TaskNote>,
    pub files: Vec<TaskFileRef>,
    pub commands: Vec<TaskCommand>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub today: Vec<TaskDetails>,
    pub backburner: Vec<TaskDetails>,
    pub promoted: usize,
}

pub fn now_string() -> String {
    Local::now().to_rfc3339()
}

pub fn today_key() -> String {
    date_key(Local::now().date_naive())
}

pub fn date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn parse_plan(value: &str) -> Result<Option<String>> {
    match value {
        "none" => Ok(None),
        "today" => Ok(Some(today_key())),
        "tomorrow" => Ok(Some(date_key(
            Local::now().date_naive() + chrono::Days::new(1),
        ))),
        other => NaiveDate::parse_from_str(other, "%Y-%m-%d")
            .map(|date| Some(date_key(date)))
            .map_err(|_| anyhow::anyhow!("plan must be today, tomorrow, none, or YYYY-MM-DD")),
    }
}

pub fn parse_timestamp(value: String) -> String {
    DateTime::parse_from_rfc3339(&value)
        .map(|date| date.to_rfc3339())
        .unwrap_or(value)
}
