# Testing Mani

Mani uses three test layers. Each layer answers a different question.

## Unit tests

Unit tests call the deep module interfaces directly. They cover Command Path
validation, path and configuration resolution, Catalog source behavior, guide
validation, outline state, scrolling, and smart-case search.

```sh
cargo test --lib
```

## End-to-end tests

The CLI tests run the compiled `mani` binary with isolated temporary files and
controlled fake commands. The TUI test opens Mani in a real pseudo-terminal,
waits for the Interactive View, sends `q`, and verifies that Mani restores the
terminal.

```sh
cargo test --test cli
cargo test --test tui
```

Run every automated test and the strict linter with:

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

## Agent tests

Agent tests ask an independent coding agent to use Mani as an end user in its
own terminal. This catches confusing behavior and terminal interactions that
assertions may miss.

Give the agent [the core-reader test brief](../tests/agent/core-reader.md). The
agent must use an isolated `MANI_HOME`, must not edit the repository, and must
report the exact commands and observed results. A pass requires both the
behavioral checks and a clean terminal after leaving the TUI.

Agent tests are evidence-based manual evaluations. They are not included in
`cargo test` because they require an independent agent session and judgment.
