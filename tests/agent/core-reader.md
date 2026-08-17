# Core-reader agent test

Act as an independent end user testing Mani. Work in the repository root, but
do not edit repository files. Do not inspect `src/` until the behavioral pass
is complete.

Record your operating system, terminal size, shell, `rustc --version`, and the
commit or working-tree state you tested. Report every command, whether it
passed, and any unexpected output or interaction.

## Setup

1. Run `cargo build`.
2. Create a temporary directory with `mktemp -d` and use it as `MANI_HOME`.
3. Create a nested Custom Guide at `<MANI_HOME>/demo/tool.md` with a title,
   introduction, at least two level-two headings, a fenced code block, and a
   table.
4. Keep all fixtures and fake executables inside temporary directories. Remove
   them when the test is complete.

## Stdout and error behavior

Verify these behaviors:

1. `mani --help` explains the core options.
2. `mani config check` reports that built-in defaults are valid when no config
   file exists.
3. `mani --short demo tool` prints only the Custom Guide title and
   introduction.
4. `mani --print demo tool` prints the complete Custom Guide.
5. `mani --print --custom missing-command` fails and prints the expected guide
   path.
6. A broken `config.toml` stops normal startup, while `--ignore-config` lets the
   same lookup succeed.
7. Redirected output contains no ANSI escapes by default. `--color always`
   adds color even when `NO_COLOR` is set. `--color never` removes it.

Create controlled fake `man` and command executables at the front of `PATH`.
Use them to verify:

1. Installed man output wins in `--print --official`.
2. Without a man page or Custom Guide, `--short` runs the exact Command Path
   with only `--help` appended.
3. Shell metacharacters passed as Command Path parts are never evaluated by a
   shell.

## Interactive View

Use a terminal at least 100 columns wide and open `mani demo tool`.

1. Confirm the header shows the Command Path, `CUSTOM`, and the exact guide
   path.
2. Scroll with `j` and `k`, then use `g` and `G`.
3. Press `o`, move through the outline, and press Enter. Confirm the document
   jumps to the selected heading.
4. Search for lowercase text with `/`, then use `n` and `N`.
5. Search again with an uppercase letter and confirm matching becomes
   case-sensitive.
6. If both sources are available, press Tab and confirm each source keeps its
   own scroll position and match count.
7. Press `q`. Confirm the normal terminal screen, cursor, echo, and line mode
   are restored.
8. Repeat in a terminal narrower than 80 columns and confirm the Minimal Pager
   replaces the outline layout.

## Report

Return:

- A pass/fail line for each numbered behavior.
- Exact commands for every failure.
- Relevant output, shortened only when it is repetitive.
- Any confusing wording or interaction, even if the strict check passed.
- A final verdict: `PASS`, `PASS WITH CONCERNS`, or `FAIL`.
