# Backburner CLI

Private project memory for work worth coming back to.

Backburner CLI stores tasks locally inside a git repository. It is intentionally
small: tasks live in `today`, `backburner`, or `archived`. Treat `today` as the
active session list: completed Today tasks archive when you run
`bb finish-session`; unfinished Today tasks return to the Backburner.

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

## Agent Context

```sh
bb context --json
bb prompt session-start
bb prompt session-end
bb prompt low-hanging-fruit
```

The CLI does not call an LLM. It stores facts and emits context that humans or
agents can interpret.

## Test

```sh
cargo test
cargo clippy -- -D warnings
```
