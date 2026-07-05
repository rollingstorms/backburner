use anyhow::Result;
use serde::Serialize;

use crate::models::{Context, TaskDetails, TaskStatus};
use crate::repository::FinishDayResult;

pub fn json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn list(title: &str, tasks: &[TaskDetails]) {
    println!("{title}");
    if tasks.is_empty() {
        println!("\nNothing here.");
        return;
    }
    println!();
    for details in tasks {
        print_task_line(details);
    }
}

pub fn show(details: &TaskDetails) {
    println!("#{} {}", details.task.id, details.task.title);
    println!("Status: {}", details.task.status);
    if let Some(planned) = &details.task.planned_date_key {
        println!("Planned: {planned}");
    }
    if details.task.completed_at.is_some() {
        println!("Completed: yes");
    }
    if !details.notes.is_empty() {
        println!("\nNotes:");
        for note in &details.notes {
            println!("- {}", note.body);
        }
    }
    if !details.files.is_empty() {
        println!("\nFiles:");
        for file in &details.files {
            match file.line {
                Some(line) => println!("- {}:{line}", file.path),
                None => println!("- {}", file.path),
            }
        }
    }
    if !details.commands.is_empty() {
        println!("\nCommands:");
        for command in &details.commands {
            println!("- {}", command.command);
        }
    }
}

pub fn context(context: &Context) {
    if context.promoted > 0 {
        println!("Promoted {} planned task(s).\n", context.promoted);
    }
    list("Today", &context.today);
    println!();
    let backburner = context
        .backburner
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    list("Backburner", &backburner);
}

pub fn finish_day(result: &FinishDayResult) {
    println!(
        "Archived {} completed task(s). Moved {} unfinished task(s) to Backburner.",
        result.archived, result.backburnered
    );
}

pub fn moved(id: i64, status: TaskStatus) {
    println!("Moved #{id} to {status}.");
}

fn print_task_line(details: &TaskDetails) {
    let check = if details.task.completed_at.is_some() {
        "[x]"
    } else {
        "[ ]"
    };
    let planned = details
        .task
        .planned_date_key
        .as_ref()
        .map(|value| format!(" ({value})"))
        .unwrap_or_default();
    println!(
        "{check} #{} {}{}",
        details.task.id, details.task.title, planned
    );
    for file in details.files.iter().take(2) {
        match file.line {
            Some(line) => println!("    {}:{line}", file.path),
            None => println!("    {}", file.path),
        }
    }
    if let Some(command) = details.commands.first() {
        println!("    {}", command.command);
    }
}
