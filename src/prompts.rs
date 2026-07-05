use anyhow::{Result, bail};

pub fn print_prompt(name: &str) -> Result<()> {
    let prompt = match name {
        "session-start" => SESSION_START,
        "session-end" => SESSION_END,
        "low-hanging-fruit" => LOW_HANGING_FRUIT,
        _ => bail!("unknown prompt '{name}'"),
    };
    println!("{prompt}");
    Ok(())
}

const SESSION_START: &str = r#"Read `bb context --json`.
Summarize the active project memory that is relevant to the user's current request.
When the request implies concrete session work, create or update Today items with useful evidence.
Use Backburner items only when they help the requested work.
Do not expand scope just because a task exists."#;

const SESSION_END: &str = r#"Review the current work, git diff, terminal output, and conversation.
Mark completed Today items done, and create or update Today items for unfinished session work before running `bb finish-session`.
Include file refs, commands, and short notes when they make the work easier to resume.
Only mark items done when code or test evidence supports it."#;

const LOW_HANGING_FRUIT: &str = r#"Read `bb backburner --json`.
Suggest up to three small, local, low-risk tasks.
Prefer clear titles, file references, and tasks that do not imply broad redesigns.
Do not start implementation without explicit user confirmation."#;
