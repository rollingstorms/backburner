# Backburner

Private project memory for work worth coming back to.

Backburner stores tasks locally inside a git repository. It is intentionally
small: tasks live in `today`, `backburner`, or `archived`. Treat `today` as the
active session list: completed Today tasks archive when you run
`bb finish-session`; unfinished Today tasks return to the Backburner.

## Architecture

Backburner is project memory, not obligation. The core model is intentionally
small:

- `today` is active memory: the current working session or daily working set.
- `backburner` is common memory: deferred, forgotten, or parked work worth
  keeping available without making it active.
- `archived` is resolved memory: work that has enough evidence to leave the
  active system.

The normal lifecycle is:

```text
today -> archived
today -> backburner -> today
```

New tasks enter Today by default. Use `--backburner` when the work is worth
keeping but should not be active yet.

`finish-session` performs the reconciliation step: completed Today tasks move
to Archive, and unfinished Today tasks move back to Backburner. Planning is not
part of the scope model; `bb plan` is only a reminder overlay that can bring a
Backburner item back into Today when it becomes relevant.

The current model assumes one active working session per checkout. Multiple
parallel sessions share the same `today` list, so they can conflict or mix
session state. Treat that as a known limitation rather than a separate status
bucket.

## Install for Development

```sh
cargo build
```

The binary is named `bb`.

## Start a Repo

```sh
bb init
```

This creates `.backburner/backburner.db` and adds `.backburner/` to
`.git/info/exclude`, keeping the memory private to your local checkout.

## Core Commands

```sh
bb add "Fix flaky login redirect"
bb add "Fix failing tests"
bb add "Park this for later" --backburner
bb today
bb backburner
bb archive
bb show 1
bb done 1
bb undone 1
bb move 1 today
bb plan 1 tomorrow
bb note 1 "Only fails after token expiry."
bb finish-session
```

`bb add` defaults to Today so capturing work keeps it in the active working set.
Use `--backburner` when a task should stay deferred.

## Emoji Aliases

Backburner accepts a small emoji vocabulary for terse automation.
Every emoji maps to an existing command or value; regular text commands remain
the human-facing interface.

```sh
bb + "Fix flaky login redirect" ☀️
bb ➕ "Park this for later" 🔥
bb add "Fix flaky login redirect" ☀️
bb add "Park this for later" 🔥
bb ☀️
bb 🔥
bb 📋
bb 👁️ 1
bb 📝 1 "Only fails after token expiry."
bb ✅ 1
bb 🟩 1
bb 🚚 1 🔥
bb move 1 🔥
bb move 1 🗄️
bb 📅 1 ☀️
bb plan 1 ☀️
bb 🌇
```

| Emoji | Meaning |
| --- | --- |
| ➕ | Add a task |
| ☀️ | Today |
| 🔥 | Backburner |
| 🗄️ | Archive |
| 📋 | Context |
| 👁️ | Show task details |
| 📝 | Add task details as a note |
| ✅ | Done |
| 🟩 | Undone |
| 🚚 | Move |
| 📅 | Plan |
| 🌇 | Finish session |

The abstraction intentionally stops at one emoji per existing concept. Emoji
aliases should not hide multi-step workflows or infer missing task ids.

## Evidence

Tasks can carry restart context:

```sh
bb add "Fix auth redirect regression" \
  --file src/auth.rs:42 \
  --cmd "cargo test auth" \
  --note "Fails after token expiry." \
  --source agent
```

## Context

```sh
bb context --json
```

Context includes Today and Backburner tasks.

## Test

```sh
cargo test
cargo clippy -- -D warnings
```
