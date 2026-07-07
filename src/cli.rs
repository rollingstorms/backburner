use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::models::{Source, TaskStatus};

const TODAY_EMOJI: &str = "☀";
const BACKBURNER_EMOJI: &str = "🔥";

#[derive(Debug, Parser)]
#[command(
    name = "bb",
    version,
    about = "Private project memory for work worth coming back to.",
    after_help = "Run `bb help --usage` for multiline workflow examples.",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize Backburner storage for this git repo.
    Init(JsonFlag),
    /// Add a task to Today.
    Add(AddArgs),
    /// List active tasks for the current session.
    Today(JsonFlag),
    /// List deferred tasks worth keeping.
    Backburner(JsonFlag),
    /// List completed archived tasks.
    Archive(JsonFlag),
    /// Show one task with notes and evidence.
    Show(IdJsonArgs),
    /// Mark a task complete.
    Done(IdArgs),
    /// Mark a task incomplete and revive archived tasks.
    Undone(IdArgs),
    /// Move a task between Today, Backburner, and Archive.
    Move(MoveArgs),
    /// Schedule a Backburner task for later.
    Plan(PlanArgs),
    /// Add restart context to a task.
    Note(NoteArgs),
    /// Delete a task permanently.
    Delete(IdArgs),
    /// Archive completed Today tasks and defer unfinished ones.
    #[command(name = "finish-session", alias = "finish-day")]
    FinishSession(JsonFlag),
    /// Print context: Today and Backburner tasks.
    Context(JsonFlag),
    /// Print command help or advanced usage examples.
    Help(HelpArgs),
}

#[derive(Debug, Args)]
pub struct JsonFlag {
    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Task title.
    pub title: String,
    /// Put the new task in Backburner instead of Today.
    #[arg(long)]
    pub backburner: bool,
    /// Kept for compatibility; Today is already the default.
    #[arg(long, hide = true)]
    pub today: bool,
    /// Reminder date such as today, tomorrow, none, or YYYY-MM-DD.
    #[arg(long)]
    pub plan: Option<String>,
    /// File evidence, optionally with a line number.
    #[arg(long = "file")]
    pub files: Vec<String>,
    /// Command evidence for restarting or verifying work.
    #[arg(long = "cmd")]
    pub commands: Vec<String>,
    /// Note to attach to the task.
    #[arg(long = "note")]
    pub notes: Vec<String>,
    /// Who created the task or note.
    #[arg(long, value_enum, default_value_t = SourceArg::Human)]
    pub source: SourceArg,
    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct IdArgs {
    /// Task id.
    pub id: i64,
}

#[derive(Debug, Args)]
pub struct IdJsonArgs {
    /// Task id.
    pub id: i64,
    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    /// Task id.
    pub id: i64,
    /// Destination status.
    pub status: StatusArg,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Task id.
    pub id: i64,
    /// Reminder date such as today, tomorrow, none, or YYYY-MM-DD.
    pub plan: String,
}

#[derive(Debug, Args)]
pub struct NoteArgs {
    /// Task id.
    pub id: i64,
    /// Note body.
    pub body: String,
    /// Who wrote the note.
    #[arg(long, value_enum, default_value_t = SourceArg::Human)]
    pub source: SourceArg,
}

#[derive(Debug, Args)]
pub struct HelpArgs {
    /// Print multiline advanced usage examples.
    #[arg(long)]
    pub usage: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StatusArg {
    Today,
    Backburner,
    Archived,
    Archive,
}

impl From<StatusArg> for TaskStatus {
    fn from(value: StatusArg) -> Self {
        match value {
            StatusArg::Today => TaskStatus::Today,
            StatusArg::Backburner => TaskStatus::Backburner,
            StatusArg::Archived | StatusArg::Archive => TaskStatus::Archived,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SourceArg {
    Human,
    Agent,
}

impl From<SourceArg> for Source {
    fn from(value: SourceArg) -> Self {
        match value {
            SourceArg::Human => Source::Human,
            SourceArg::Agent => Source::Agent,
        }
    }
}

pub fn normalize_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut args: Vec<String> = args.into_iter().collect();
    normalize_top_level_command(&mut args);
    normalize_command_values(&mut args);
    args
}

fn normalize_top_level_command(args: &mut [String]) {
    if let Some(command) = args.get_mut(1) {
        if let Some(normalized) = emoji_command(command) {
            *command = normalized.to_owned();
        }
    }
}

fn normalize_command_values(args: &mut Vec<String>) {
    match args.get(1).map(String::as_str) {
        Some("add") => normalize_add_args(args),
        Some("move") => normalize_move_args(args),
        Some("plan") => normalize_plan_args(args),
        _ => {}
    }
}

fn normalize_add_args(args: &mut Vec<String>) {
    match args.last().map(|arg| emoji_key(arg)) {
        Some(key) if key == TODAY_EMOJI => {
            args.pop();
        }
        Some(key) if key == BACKBURNER_EMOJI => {
            if let Some(last) = args.last_mut() {
                *last = "--backburner".to_owned();
            }
        }
        _ => {}
    }
}

fn normalize_move_args(args: &mut [String]) {
    if let Some(status) = args.get_mut(3) {
        if let Some(normalized) = emoji_status(status) {
            *status = normalized.to_owned();
        }
    }
}

fn normalize_plan_args(args: &mut [String]) {
    if let Some(plan) = args.get_mut(3) {
        if emoji_key(plan) == TODAY_EMOJI {
            *plan = "today".to_owned();
        }
    }
}

fn emoji_command(value: &str) -> Option<&'static str> {
    match emoji_key(value).as_str() {
        "+" | "➕" => Some("add"),
        TODAY_EMOJI => Some("today"),
        BACKBURNER_EMOJI => Some("backburner"),
        "📰" | "🗄" => Some("archive"),
        "📋" => Some("context"),
        "📓" | "👁" => Some("show"),
        "📝" => Some("note"),
        "✅" => Some("done"),
        "🟩" => Some("undone"),
        "👉" | "🚚" => Some("move"),
        "📅" => Some("plan"),
        "🌇" => Some("finish-session"),
        _ => None,
    }
}

fn emoji_status(value: &str) -> Option<&'static str> {
    match emoji_key(value).as_str() {
        TODAY_EMOJI => Some("today"),
        BACKBURNER_EMOJI => Some("backburner"),
        "📰" | "🗄" | "✅" => Some("archive"),
        _ => None,
    }
}

fn emoji_key(value: &str) -> String {
    value.replace('\u{fe0f}', "")
}
