# Mani implementation specification

Status: implementation-ready design, confirmed 2026-08-17.

This specification turns the project glossary and ADRs into a build contract. The glossary in [`CONTEXT.md`](../CONTEXT.md) defines canonical product language; the ADRs under [`docs/adr/`](./adr/) explain durable choices and take precedence if this document drifts.

## Product definition

Mani is a customizable, interactive alternative to the Unix `man` command. It puts concise, personally curated Markdown help beside authoritative local command documentation and lets users switch between them without leaving the terminal.

Mani is offline-first. Reading Custom Guides and Official Documentation never requires a model, provider account, or network connection. Model-assisted authoring is optional.

## V1 scope

V1 supports:

- A Rust binary on macOS and Linux.
- Custom Guides stored as portable Markdown.
- Installed man pages with safe `<command> --help` fallback.
- A full-screen, keyboard-driven TUI.
- Complete and concise stdout modes.
- Optional draft creation and refinement through a configured local command.
- Optional Zsh and Bash hooks for recent command lines and exit codes.
- Local, privacy-preserving detection of recurring issues.
- Diff-based history for changes Mani makes to Custom Guides.

V1 does not include:

- Native Windows support.
- Shell hooks for shells other than Zsh and Bash.
- Automatic terminal-output recording.
- Raw HTML, ANSI escapes, or Mani-specific frontmatter in Custom Guides.
- A built-in text editor.
- Built-in HTTP/SDK integrations with specific LLM providers; LLM generation is supported through configurable Generator Commands.
- Application-level encryption.
- Persistent storage of prompts, raw Diagnostic Output, or generated partials.

## Command-line interface

### Help lookup

```text
mani [options] <command> [subcommand ...]
```

Every positional token is part of the Command Path. Lookup does not accept command options or operands; concrete failed invocations belong to `mani explain`.

Examples:

```sh
mani git
mani git rebase
mani docker compose up
```

### Output and source selection

```text
--short                 Print concise help and do not open the TUI
--print                 Print a complete document and do not open the TUI
--custom                Select the Custom Guide
--official              Select Official Documentation
--color auto|always|never
--ignore-config         Use built-in defaults when config.toml is broken
```

`--custom` and `--official` are mutually exclusive. They select the initial source in Interactive View and the complete document used by `--print`. `--custom` fails when its guide is absent rather than silently switching sources. `--short` has fixed source rules and rejects either source override.

### Supporting commands

```text
mani explain [<command> [subcommand ...]]
mani setup
mani shell init zsh|bash
mani shell status
mani config check
mani state summary|recent|issues
mani state clear recent|issues|all [--yes]
mani history <command> [subcommand ...]
mani history restore <revision> <command> [subcommand ...]
mani history clear <command> [subcommand ...]
```

`mani explain` reads Diagnostic Output from piped stdin when present. It uses the Shell Integration's Relevant Command when available; otherwise the Interactive View asks for the Command Path. It never reruns the failed command.

## Files and paths

### Knowledge Base Root

Resolution order:

1. `MANI_HOME`, when set.
2. `$XDG_CONFIG_HOME/mani`, when `XDG_CONFIG_HOME` is set.
3. `~/.config/mani`.

The root contains configuration and shareable content:

```text
~/.config/mani/
├── config.toml
├── git.md
├── git/
│   └── rebase.md
├── docker/
│   └── compose/
│       └── up.md
└── .mani/
    └── history/
        └── git/
            └── rebase/
                └── <timestamp>-<hash>.patch
```

A Command Path maps directly to its Markdown path. For example, `docker compose up` maps to `docker/compose/up.md`.

### Private State Root

Resolution order:

1. `$XDG_STATE_HOME/mani`, when `XDG_STATE_HOME` is set.
2. `~/.local/state/mani`.

The directory is created with mode `0700`. Its SQLite database is created with mode `0600` and uses WAL mode.

```text
~/.local/state/mani/
└── state.sqlite3
```

The database contains only recent command metadata, Issue Records, policy state, and schema migrations. It never contains raw Diagnostic Output, prompts, Custom Guides, or Generated Drafts.

## Configuration

Mani reads `config.toml` from the Knowledge Base Root. Unknown keys and invalid values fail with the file, line, failing value, and a suggested correction. `mani config check` performs validation without launching help. `--ignore-config` is the recovery path.

Initial shape:

```toml
[generator]
command = [
  "codex", "exec",
  "--ephemeral",
  "--sandbox", "read-only",
  "--skip-git-repo-check",
  "--ignore-user-config",
  "--ignore-rules",
  "--model", "gpt-5.6-luna",
  "--config", "model_reasoning_effort=\"low\"",
  "-",
]
inactivity_warning_seconds = 35
timeout_seconds = 270

[history]
limit = 20 # 0..=100; 0 disables history

[issues]
refine_after = 3
window_days = 30
reprompt_multiplier = 2

[theme]
heading = "cyan"
code = "yellow"
warning = "yellow bold"
active_source = "green reverse"
muted = "dim"
```

The Codex command is an auto-detected preset, not a requirement. Setup shows the selected command and writes it only after confirmation. A user may replace the entire argument array with any command that obeys the Generator Command contract.

## Help resolution

For a Command Path, the catalog resolves both sources independently.

### Custom source

1. Map the Command Path to its Markdown path under the Knowledge Base Root.
2. Read the file as UTF-8.
3. Reject raw ANSI escapes and raw HTML.
4. Parse CommonMark with fenced code blocks and tables.

### Official source

1. Convert the full Command Path to the platform's man topic form, such as `git rebase` to `git-rebase`.
2. Prefer the installed man page when found.
3. Otherwise execute the exact Command Path with only `--help` appended.
4. Build the process argument vector directly without shell interpolation.
5. Close stdin, capture stdout and stderr, and apply a short implementation-defined timeout.
6. Label the source exactly, such as `man git-rebase(1)` or `git rebase --help`.

Mani may strip control codes, normalize layout, wrap lines, detect headings, and build an outline. It must preserve Official Documentation wording, order, and omissions exactly.

### Missing sources

When no source exists, Interactive View opens a recovery screen showing the Command Path and offers `c` to create a Generated Draft. If generation is unavailable, it shows the exact Custom Guide path to create manually.

`--short` and `--print` write a clear error to stderr and exit nonzero when their required source cannot be resolved.

## Document and output rules

Custom Guides use CommonMark plus fenced code blocks and tables. Heading order defines document grouping and order. The Theme controls all presentation.

`--short` behaves as follows:

- With a Custom Guide, print its title and introductory content before the first level-two heading.
- Without a Custom Guide, print safely captured command-generated `--help`.
- Do not substitute a man page for this concise fallback.

`--print` writes the complete selected document. It prefers the Custom Guide when both sources exist.

Color defaults to `auto`: use color when stdout is a terminal and plain text when redirected. `NO_COLOR` forces plain text unless the user explicitly selects `--color=always`.

## Interactive View

The default layout is the Outline Browser. It has:

- A header with the Command Path, active source, and exact source location.
- An outline on the left and the active document on the right.
- A compact footer showing context-relevant keys.
- A Minimal Pager fallback when the terminal is too narrow for the outline.

When a Custom Guide exists, Mani opens Custom Mode. Otherwise it opens Official Mode immediately and keeps the source indicator visible. It never starts generation automatically. An explicit `--custom` or `--official` override selects the initial source.

### Navigation

```text
Tab                 Switch Custom and Official modes
j/k or Up/Down      Scroll
Page Up/Page Down   Scroll by page
g/G                 Jump to top/bottom
o                   Focus the outline
Enter               Jump to the selected outline heading
Esc                 Leave outline, close a panel, or cancel the current action
/                   Start plain-text smart-case search
n/N                 Next/previous match
c                   Create a Generated Draft when no Custom Guide exists
r                   Refine an existing Custom Guide
a                   Accept and save a reviewed Generated Draft
e                   Edit a complete draft or Custom Guide in an external editor
q                   Quit
```

The outline tracks the visible heading during normal scrolling. In outline focus, arrows or `j`/`k` select a heading and `Enter` jumps to it.

Search is case-insensitive unless the query contains an uppercase letter. It searches only the active source, preserves the query across `Tab`, reruns it in the other source, and reports match counts separately. Regex search is not part of v1.

Custom and Official modes maintain independent scroll positions, active headings, and current matches.

### External editing

`e` opens a temporary Markdown file with `$VISUAL`, then `$EDITOR`, then `vi`. When the editor exits, Mani validates the file and returns to the rendered review. Editing never writes the Custom Guide directly.

## Draft generation and refinement

### Generation Context

The review screen shows the exact bundle before the Generator Command starts. The allowed fields are:

- Command Path.
- Official Documentation, when available.
- OS, shell, and tool versions.
- One Relevant Command, when a local match clears the relevance threshold.
- Diagnostic Output explicitly pasted or piped by the user.
- The existing Custom Guide during refinement only.

No working-directory path, environment value, automatically captured terminal output, unrelated recent command, or unrelated file content may enter the bundle.

The Relevant Command is chosen locally from the ten most recent commands using fuzzy Command Path similarity, recency, and failure status. Only the highest-scoring candidate is shown, collapsed by default. It is unselected for normal creation or refinement and preselected for `mani explain`. A weak match is omitted.

The user confirms the complete bundle before execution.

### Generator Command contract

- Approved context is written to stdin.
- Final Markdown is written to stdout.
- Progress and diagnostics are written to stderr.
- Exit code zero means success.
- Nonzero exit, empty stdout, invalid UTF-8, raw ANSI, raw HTML, or invalid guide content means failure.

The command runs from an isolated empty temporary working directory. The command is user-trusted and executes with the user's OS identity; Mani does not claim to sandbox arbitrary configured executables beyond the controls supplied by that executable.

### Runtime behavior

- Render stdout as a read-only live Markdown preview.
- Disable `a` and `e` until the process exits zero and the full document validates.
- Treat stdout or stderr activity as progress.
- After 35 seconds without activity, show an inactivity warning and remaining time.
- After 270 seconds total, terminate the process.
- Let `Esc` interrupt immediately, then terminate after a short grace period.
- Keep stderr in a collapsible error panel.
- Discard partial output after cancellation, timeout, or failure.
- Never modify the existing Custom Guide on failure.

When Official Documentation is absent, creation remains available but the context review and preview show `No official source available`. Generated text must never imply an official citation that does not exist.

### Review and save

A successful output is a Generated Draft, not a Custom Guide. The user may edit, accept, or discard it.

Before replacement, Mani shows a diff. Accepting performs an atomic write. If a guide already exists and history is enabled, Mani records the reverse diff before replacement and prunes old history only after the new write succeeds.

## Guide Revisions

Mani records only replacements it performs. It does not watch or reconstruct direct edits made outside Mani.

Each Guide Revision contains:

- Timestamp.
- Old and new content hashes.
- Reverse unified diff needed to restore the previous guide.

It contains no prompt, Diagnostic Output, or Generation Context. Retention defaults to 20 per guide and is configurable from 0 through 100. Restoring an old revision first records the current guide as a new revision, making restoration undoable. Hash mismatches stop restoration without changing the guide.

## Shell Integration and explanation

The optional Zsh and Bash hooks retain only the ten most recent command lines and exit codes. They never record terminal output.

`mani setup`:

1. Explains exactly what the hook records.
2. Allows the user to skip it.
3. Shows the exact `.zshrc` or `.bashrc` change.
4. Offers manual instructions.
5. After explicit confirmation, creates a backup and appends one marked, idempotent line.

Users provide Diagnostic Output by pasting it in Interactive View or piping it:

```sh
some-command 2>&1 | mani explain
```

The raw output exists only for that session and is discarded afterward.

## Recurring issues

Mani detects similar failures locally without a model. It normalizes Diagnostic Output by removing changing values such as paths, timestamps, line numbers, hashes, and IDs, then fingerprints:

- Command Path.
- Exit code, when available.
- Normalized Diagnostic Output.

An Issue Record stores only the fingerprint, a sanitized label, occurrence count, last-seen date, and refinement-prompt policy state.

The default policy offers refinement after three similar failures within 30 days. After dismissal, it asks again only when the guide changes or the occurrence count doubles. The thresholds are configuration, not part of fingerprinting.

## Module design

Production code should concentrate behavior behind five deep modules. Their Interfaces are also the primary test surfaces.

### Catalog module

Interface:

```rust
resolve(command_path: CommandPath) -> Result<HelpTopic, CatalogError>
```

`HelpTopic` contains independently optional Custom and Official documents with exact source labels. The module hides filesystem mapping, man topic lookup, safe `--help` execution, control-code cleanup, Markdown parsing, heading extraction, and short-output selection.

Deleting this module would spread source precedence, path mapping, process safety, and document normalization across the CLI, TUI, and authoring flows; that complexity is why the module earns its depth.

### Viewer module

Interface:

```rust
Viewer::new(topic: HelpTopic, initial_source: SourceKind) -> Viewer
Viewer::update(&mut self, input: ViewerInput) -> ViewerEffect
Viewer::frame(&self, area: Rect) -> ViewerFrame
```

The module owns source switching, independent positions, outline focus, smart-case search, responsive layout selection, overlays, and key availability. Ratatui/Crossterm are adapters at the terminal seam; tests use Ratatui's test backend and direct `ViewerInput` values.

### Authoring module

Interface:

```rust
prepare(command_path: CommandPath, draft: ValidGuide) -> Result<ChangeReview, AuthoringError>
save(review: ConfirmedChange) -> Result<SaveOutcome, AuthoringError>
restore(command_path: CommandPath, revision: RevisionId) -> Result<SaveOutcome, AuthoringError>
```

The module hides temporary editor files, validation, diff creation, atomic replacement, revision retention, restoration hashes, and pruning. Callers cannot write a guide without passing through review and validation types.

### Assistant module

Interface:

```rust
prepare(request: DraftRequest) -> Result<ContextReview, AssistantError>
start(approved: ApprovedContext) -> Result<GenerationRun, AssistantError>
```

`GenerationRun` yields typed stdout-preview, stderr-progress, warning, completion, cancellation, and failure events. The module hides context filtering, prompt construction, process lifetime, inactivity and hard timeouts, output validation, and missing-official warnings.

The process runner is a real seam with two adapters: a production child-process adapter and a deterministic fake for tests.

### Activity module

Interface:

```rust
observe(command: CommandObservation) -> Result<(), ActivityError>
relevant(command_path: &CommandPath) -> Result<Option<RelevantCommand>, ActivityError>
record_issue(observation: IssueObservation) -> Result<RefinementAdvice, ActivityError>
inspect(query: StateQuery) -> Result<StateView, ActivityError>
clear(target: ClearTarget) -> Result<ClearPreview, ActivityError>
```

The module hides SQLite migrations, WAL configuration, command retention, fuzzy ranking, normalization, fingerprints, issue counts, and refinement policy. Tests use a temporary SQLite database through the same Interface; no repository trait is exposed merely for mocking.

### Composition

The CLI root loads paths and configuration, constructs the modules, translates parsed commands into module calls, and renders returned outcomes. It contains no source-resolution, authoring, generation, or issue-detection rules.

Use internal seams only where behavior truly varies:

- Process runner: production and fake adapters.
- Clock: system and deterministic test adapters.
- Terminal: Crossterm and Ratatui test adapters.
- Filesystem and SQLite: real temporary local substitutes in tests rather than public repository interfaces.

## Verification strategy

Tests should assert behavior through module Interfaces and survive internal refactors.

### Catalog

- Command Path to guide-path mapping, including nested commands.
- Custom/Official precedence and missing-source outcomes.
- Man-page preference and exact `--help` argv without a shell.
- Official wording preservation after normalization.
- Short and complete stdout selection.

### Viewer

- Golden frames for normal, narrow, missing-guide, generation, and error states.
- Source switching with independent positions and matches.
- Outline focus and heading jumps.
- Smart-case search and match counts.
- Terminal restoration after quit, error, panic, and cancellation.

### Authoring and Assistant

- Forbidden guide content and invalid generator output.
- Live stdout/stderr event handling.
- 35-second warning and configurable 270-second timeout using a fake clock/process.
- Cancellation escalation and partial-output discard.
- Diff review, atomic save, pruning, restoration, and hash mismatch refusal.

### Activity and shell hooks

- Concurrent SQLite writes from multiple processes.
- Ten-command retention and minimum relevance threshold.
- Secret-shaped and changing-value normalization fixtures.
- Three-in-30-days refinement policy and dismissal cooldown.
- Zsh and Bash hook output, idempotent installation, backup, and status detection.
- State inspection and confirmed deletion.

### End-to-end

- CLI contract and exit behavior on macOS and Linux.
- TTY versus piped color behavior, including `NO_COLOR`.
- Operation with no config, no Custom Guide, no man page, and no Generator Command.
- Core reader operation with the network unavailable.

## Delivery slices

### Slice 1: Core reader

Deliver:

- Rust workspace and CLI parsing.
- Configuration and path resolution.
- Catalog module.
- Custom and Official document loading.
- Outline Browser and Minimal Pager.
- Search, source switching, and independent positions.
- `--short`, `--print`, source overrides, and color rules.

Done when the core reader works offline on macOS and Linux and all Slice 1 behavior is covered at module Interfaces.

### Slice 2: Authoring

Deliver:

- Generator Command configuration and Codex preset detection.
- Generation Context review.
- Live preview, warnings, timeout, cancellation, and validation.
- External editing, diff review, atomic save, and Guide Revisions.

Done when a user can create, refine, review, edit, accept, restore, and discard guides without any failed path changing an existing guide.

### Slice 3: Context

Deliver:

- Private SQLite state and migrations.
- Zsh/Bash hook generation, setup, backup, and status.
- Relevant Command scoring.
- `mani explain` and explicit Diagnostic Output.
- Issue fingerprints, recurrence policy, and state controls.

Done when recurring failures can prompt a reviewed refinement without raw terminal output or unrelated shell context being retained or sent.

## Definition of done for v1

V1 is complete when all three slices pass their acceptance checks on macOS and Linux, the Python prototype is no longer needed to understand production behavior, and every externally visible behavior in this specification is either implemented or explicitly deferred through a new ADR.
