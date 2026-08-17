# Mani

Mani is a customizable, interactive alternative to the Unix/POSIX `man` command. It presents command-line help from a personal knowledge base and official command documentation.

## Language

**Custom Knowledge Base**:
A user-controlled collection of Markdown help documents whose explanations, ordering, grouping, formatting, and color presentation reflect the user's preferences.
_Avoid_: Personal knowledge base, custom docs

**Knowledge Base Root**:
The directory that contains a user's Custom Guides and mirrors their Command Paths.
_Avoid_: Mani home, docs directory

**Private State Root**:
The machine-local directory that contains recent-command metadata and Issue Records and is not intended for dotfiles synchronization.
_Avoid_: State directory, private history

**Mani Configuration**:
The optional `config.toml` file at the Knowledge Base Root that defines user preferences such as the Generator Command.
_Avoid_: Settings file, JSON config

**Theme**:
The global mapping from semantic presentation roles—such as headings, code, warnings, active source, and muted text—to terminal styles.
_Avoid_: Color scheme, guide styling

**Command Path**:
The ordered command and subcommands that identify a help topic, such as `docker compose up`.
_Avoid_: Topic ID, document key

**Custom Guide**:
The portable CommonMark document for one Command Path in the Custom Knowledge Base. It may use fenced code blocks and tables, while its visual presentation belongs to Mani's theme.
_Avoid_: Custom page, knowledge entry

**Guide Revision**:
A restorable, diff-based record of a Custom Guide replacement performed by Mani.
_Avoid_: Backup, version, snapshot

**Generated Draft**:
An ephemeral, model-generated proposal for a Custom Guide that has not been reviewed or accepted by the user.
_Avoid_: Generated guide, AI documentation

**Generator Command**:
A configurable local command that receives the approved Generation Context on standard input and returns a Generated Draft as Markdown on standard output.
_Avoid_: LLM provider, model backend

**Generation Context**:
The user-approved information supplied to an LLM to create a Generated Draft: the Command Path, Official Documentation, OS, shell and tool versions, selected recent commands, and explicitly provided Diagnostic Output. Refinement also includes the existing Custom Guide.
_Avoid_: Shell context, prompt context

**Diagnostic Output**:
Command output that the user explicitly pastes or pipes into Mani to explain a failure.
_Avoid_: Terminal history, captured session

**Issue Record**:
A local, inspectable summary of a recurring command failure containing its Command Path, exit code, sanitized fingerprint and label, occurrence count, and last-seen date, but no raw Diagnostic Output.
_Avoid_: Error log, shell history

**Relevant Command**:
The highest-scoring command among the ten most recent shell commands, ranked locally by fuzzy similarity to the Command Path, recency, and failure status.
_Avoid_: Matched command, related history

**Shell Integration**:
An optional shell hook that gives Mani recent command lines and their exit codes without recording terminal output.
_Avoid_: Terminal integration, session recorder

**Custom Mode**:
The help view backed by the Custom Knowledge Base.
_Avoid_: Personal mode, knowledge-base mode

**Official Documentation**:
The locally installed man page for a Command Path, or its command-generated `--help` output when no man page exists.
_Avoid_: Source of truth, upstream docs

**Official Mode**:
The help view backed by Official Documentation.
_Avoid_: Fallback mode, docs mode

**Interactive View**:
The default full-screen help experience, with persistent navigation, scrolling, and switching between Custom Mode and Official Mode.
_Avoid_: Full-screen mode, pager mode

**Short Output**:
The concise, non-interactive help written to standard output by `--short`: a Custom Guide's title and introduction, or command-generated `--help` when no guide exists.
_Avoid_: Print mode, summary mode

**Printed Output**:
The complete, non-interactive help document written to standard output by `--print`.
_Avoid_: Full output, inline mode
