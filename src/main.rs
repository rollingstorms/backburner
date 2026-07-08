mod cli;
mod db;
mod models;
mod output;
mod repository;
mod root;
mod settings;

use std::env;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command, SessionCommand};
use crate::models::{TaskStatus, parse_plan};
use crate::repository::{CreateTask, Repository, parse_file_ref};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from(env::args());
    match cli.command {
        Command::Help(args) => {
            if args.usage {
                print_advanced_usage();
                Ok(())
            } else {
                Cli::command().print_help()?;
                println!();
                Ok(())
            }
        }
        Command::Init(args) => {
            let cwd = env::current_dir()?;
            let root = root::find_git_root(&cwd)?;
            let db_path = root::init_project(&root)?;
            settings::init(&root)?;
            let conn = db::open(&db_path)?;
            drop(conn);
            if args.json {
                output::json(&serde_json::json!({
                    "root": root,
                    "database": db_path,
                }))
            } else {
                println!("Initialized Backburner at {}.", db_path.display());
                Ok(())
            }
        }
        other => run_project_command(other),
    }
}

fn run_project_command(command: Command) -> Result<()> {
    let cwd = env::current_dir()?;
    let root = root::find_git_root(&cwd)?;
    let db_path = root::ensure_initialized(&root)?;
    let conn = db::open(&db_path)?;
    let mut repository = Repository::new(conn);
    settings::rollover_if_needed(&root, &mut repository)?;

    match command {
        Command::Add(args) => {
            let planned_date_key = args.plan.as_deref().map(parse_plan).transpose()?.flatten();
            let active_session = settings::active_session(&root)?;
            let details = repository.create(CreateTask {
                title: args.title,
                status: if args.backburner {
                    TaskStatus::Backburner
                } else {
                    TaskStatus::Today
                },
                planned_date_key,
                session_key: active_session,
                source: args.source.into(),
                notes: args.notes,
                files: args
                    .files
                    .iter()
                    .map(|value| parse_file_ref(value))
                    .collect(),
                commands: args.commands,
            })?;
            if args.json {
                output::json(&details)
            } else {
                output::show(&details);
                Ok(())
            }
        }
        Command::Today(args) => {
            let active_session = settings::active_session(&root)?;
            repository.promote_due_for(active_session.as_deref())?;
            let tasks =
                repository.list_for_session(TaskStatus::Today, active_session.as_deref())?;
            if args.json {
                output::json(&tasks)
            } else {
                output::list("Today", &tasks);
                Ok(())
            }
        }
        Command::Backburner(args) => {
            let active_session = settings::active_session(&root)?;
            let tasks =
                repository.list_for_session(TaskStatus::Backburner, active_session.as_deref())?;
            if args.json {
                output::json(&tasks)
            } else {
                output::list("Backburner", &tasks);
                Ok(())
            }
        }
        Command::Archive(args) => {
            let tasks = repository.list(TaskStatus::Archived)?;
            if args.json {
                output::json(&tasks)
            } else {
                output::list("Archive", &tasks);
                Ok(())
            }
        }
        Command::Show(args) => {
            let details = repository.details(args.id)?;
            if args.json {
                output::json(&details)
            } else {
                output::show(&details);
                Ok(())
            }
        }
        Command::Done(args) => {
            repository.set_completed(args.id, true)?;
            println!("Marked #{} done.", args.id);
            Ok(())
        }
        Command::Undone(args) => {
            let status = repository.mark_undone(args.id)?;
            println!("Marked #{} undone in {}.", args.id, status);
            Ok(())
        }
        Command::Move(args) => {
            let status = args.status.into();
            repository.move_to(args.id, status)?;
            output::moved(args.id, status);
            Ok(())
        }
        Command::Plan(args) => {
            repository.plan(args.id, parse_plan(&args.plan)?)?;
            println!("Updated plan for #{}.", args.id);
            Ok(())
        }
        Command::Note(args) => {
            repository.add_note(args.id, &args.body, args.source.into())?;
            println!("Added note to #{}.", args.id);
            Ok(())
        }
        Command::Delete(args) => {
            repository.delete(args.id)?;
            println!("Deleted #{}.", args.id);
            Ok(())
        }
        Command::FinishSession(args) => {
            let active_session = settings::active_session(&root)?;
            let session = args.session.as_deref().or(active_session.as_deref());
            let result = repository.finish_session_for(session)?;
            if args.json {
                output::json(&result)
            } else {
                output::finish_session(&result);
                Ok(())
            }
        }
        Command::Context(args) => {
            let active_session = settings::active_session(&root)?;
            let context = repository.context_for(active_session.as_deref())?;
            if args.json {
                output::json(&context)
            } else {
                output::context(&context);
                Ok(())
            }
        }
        Command::Session(args) => match args.command {
            SessionCommand::Start(args) => {
                let name = settings::start_session(&root, &args.name)?;
                println!("Started session {name}.");
                Ok(())
            }
            SessionCommand::End => {
                match settings::end_session(&root)? {
                    Some(name) => println!("Ended session {name}."),
                    None => println!("No active session."),
                }
                Ok(())
            }
        },
        Command::Init(_) | Command::Help(_) => unreachable!(),
    }
}

fn print_advanced_usage() {
    println!(
        r#"Backburner advanced usage

Start a repo:
  bb init

Capture active work:
  bb add "Investigate flaky login redirect"
  bb add "Fix failing auth test"
  bb add "Park this for later" --backburner

Attach restart evidence:
  bb add "Fix auth redirect regression" \
    --file src/auth.rs:42 \
    --cmd "cargo test auth" \
    --note "Fails after token expiry." \
    --source agent

Review and move work:
  bb today
  bb backburner
  bb show 1
  bb move 1 today
  bb plan 1 tomorrow

Close a session:
  bb done 1
  bb finish-session
  bb finish-session refactor-auth
  bb session end

Context:
  bb context --json"#
    );
}
