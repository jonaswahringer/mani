# Build the production CLI in Rust

Mani's production implementation is a Rust binary, using Ratatui with Crossterm for the full-screen terminal interface and `pulldown-cmark` for portable Markdown parsing. Rust provides a fast, self-contained executable suitable for frequent shell use across Unix-like systems; the Python TUI remains a throwaway design prototype rather than an implementation base.
