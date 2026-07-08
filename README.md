# Backburner

Private project memory for work worth coming back to.

> Not everything belongs today.
>
> Put it on the backburner.

Backburner stores tasks locally inside a git repository. It is intentionally
small. Every task lives in one of three places: **Today**, **Backburner**, or
**Archive**.

Backburner is named after the kitchen. When you're busy on the line, you move
something to the back burner—not because it isn't important, but because
something else needs your attention first. It'll still be there when you're
ready.

## The Model

Backburner has three states.

- **Today** — your active working memory.
- **Backburner** — deferred work waiting to become relevant again.
- **Archive** — resolved work that is no longer on the docket.

```text
Today ─────► Archive
   │
   ▼
Backburner ─────► Today
```

New tasks enter Today by default. Use `--backburner` to park work without
making it active.

`bb finish-session` reconciles your working memory.

- Completed tasks move to Archive.
- Unfinished Today tasks return to the Backburner.

Tomorrow starts with a clean Today.

## Install

```sh
cargo install backburner
```

The installed binary is named `bb`.

Or use the shell installer:

```sh
curl -fsSL https://raw.githubusercontent.com/rollingstorms/backburner/main/install.sh | sh
```

### Developer Build

Build from source when working on Backburner itself:

```sh
git clone https://github.com/rollingstorms/backburner.git
cd backburner
cargo build --release
```

## Initialize a Repository

```sh
bb init
```

This creates `.backburner/backburner.db` and adds `.backburner/` to
`.git/info/exclude`, keeping your project memory private to your local checkout.

## Commands

### Capture

```sh
bb add "Fix flaky login redirect"
bb add "Park this for later" --backburner
```

### View

```sh
bb today
bb backburner
bb archive
bb show 1
```

### Update

```sh
bb done 1
bb undone 1
bb move 1 today
bb plan 1 tomorrow
bb note 1 "Only fails after token expiry."
```

### Sessions

Sessions scope the Today list.

`bb session start <name>` creates or resumes a working session. New tasks are
added to that session's Today list.

`bb finish-session` reconciles the active session. When no session is active,
it reconciles every Today task.

Use `bb finish-session <name>` to reconcile a specific session regardless of
which session is currently active.

```sh
bb session start refactor-auth
bb session end
bb finish-session
bb finish-session refactor-auth
```

## Context

Tasks can carry context that makes them easier to resume later.

```sh
bb add "Fix auth redirect regression" \
  --file src/auth.rs:42 \
  --cmd "cargo test auth" \
  --note "Fails after token expiry." \
  --source agent
```

Machine-readable context is also available:

```sh
bb context --json
```

`bb context` includes Today and Backburner tasks.

## Test

```sh
cargo test
cargo clippy -- -D warnings
```
