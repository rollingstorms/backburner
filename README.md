# Backburner CLI

Private project memory for work worth coming back to.

Backburner CLI stores tasks locally inside a git repository. It is intentionally
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
backburner -> today -> archived
             today -> backburner
```

`finish-session` performs the reconciliation step: completed Today tasks move
to Archive, and unfinished Today tasks move back to Backburner. Planning is not
part of the scope model; `bb plan` is only a reminder overlay that can promote a
Backburner item into Today when it becomes relevant.

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
bb add "Fix failing tests" --today
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

`bb add` defaults to Backburner so capturing an idea does not interrupt the
current session. Use `--today` when a task belongs in the active working set.

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
