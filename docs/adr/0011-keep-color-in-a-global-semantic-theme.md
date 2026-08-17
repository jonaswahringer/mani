# Keep color in a global semantic theme

Mani defines terminal presentation through one global Theme in `config.toml`, using semantic roles such as headings, code, warnings, active source, and muted text. Custom Guides contain no per-document colors, and rendering respects terminal capabilities and `NO_COLOR`, keeping guides portable and the interface consistent. `--short` and `--print` use `--color=auto` by default, emit plain text when stdout is redirected, and accept `always` or `never`; `NO_COLOR` forces plain text unless the user explicitly requests `always`.
