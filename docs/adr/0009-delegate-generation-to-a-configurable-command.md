# Delegate generation to a configurable command

Mani delegates draft generation to a configurable Generator Command, passing the approved context on standard input and reading Markdown from standard output. When a supported tool is installed, Mani may offer a visible, overridable preset; the initial Codex preset uses an ephemeral, read-only `codex exec` invocation with `gpt-5.6-luna` at low reasoning effort. If no known tool or explicit command is available, generation stays disabled while core help continues to work offline.
