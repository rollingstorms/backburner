use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::models::{Source, TaskStatus};

#[derive(Debug, Parser)]
#[command(
    name = "bb",
    version,
    about = "Private project memory for work worth coming back to."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(JsonFlag),
    Add(AddArgs),
    Today(JsonFlag),
    Backburner(JsonFlag),
    Archive(JsonFlag),
    Show(IdJsonArgs),
    Done(IdArgs),
    Undone(IdArgs),
    Move(MoveArgs),
    Plan(PlanArgs),
    Note(NoteArgs),
    Delete(IdArgs),
    #[command(name = "finish-session", alias = "finish-day")]
    FinishSession(JsonFlag),
    Context(JsonFlag),
    Prompt(PromptArgs),
}

#[derive(Debug, Args)]
pub struct JsonFlag {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    pub title: String,
    #[arg(long)]
    pub today: bool,
    #[arg(long)]
    pub plan: Option<String>,
    #[arg(long = "file")]
    pub files: Vec<String>,
    #[arg(long = "cmd")]
    pub commands: Vec<String>,
    #[arg(long = "note")]
    pub notes: Vec<String>,
    #[arg(long, value_enum, default_value_t = SourceArg::Human)]
    pub source: SourceArg,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct IdArgs {
    pub id: i64,
}

#[derive(Debug, Args)]
pub struct IdJsonArgs {
    pub id: i64,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    pub id: i64,
    pub status: StatusArg,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    pub id: i64,
    pub plan: String,
}

#[derive(Debug, Args)]
pub struct NoteArgs {
    pub id: i64,
    pub body: String,
    #[arg(long, value_enum, default_value_t = SourceArg::Human)]
    pub source: SourceArg,
}

#[derive(Debug, Args)]
pub struct PromptArgs {
    pub name: String,
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
