# mani

Mani is a customizable, interactive alternative to the Unix `man` command. It
puts concise, personally curated Markdown help beside authoritative local
command documentation and lets users switch between them without leaving the
terminal.

The product design is complete. Production implementation is underway in Rust.
The first core-reader foundation now includes command parsing, configuration
and path resolution, safe local help lookup, stdout modes, and the initial
Ratatui viewer state.

## Build and run the production binary

Mani requires Rust 1.85 or newer.

```sh
cargo build
cargo run -- git rebase
```

Non-interactive output is available now:

```sh
cargo run -- --short git rebase
cargo run -- --print --official git rebase
cargo run -- config check
```

Run the current checks with:

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The full unit, end-to-end, and independent agent-testing workflow is described
in [docs/testing.md](docs/testing.md).

Slice 2 authoring commands and Slice 3 context commands are not implemented
yet. The complete delivery plan and behavior contract remain in the
[implementation specification](docs/implementation-spec.md).

## Design documentation

- [Implementation specification](docs/implementation-spec.md)
- [Domain language](CONTEXT.md)
- [Architecture decisions](docs/adr/)

## Historical TUI prototype

```sh
./prototype/mani.py git rebase
```

Inside the prototype:

- `Tab` switches between Custom and Official sources.
- `←` and `→` switch among three layout variants.
- `↑`, `↓`, `j`, `k`, `Page Up`, and `Page Down` scroll.
- `q` quits.

To exercise the non-interactive path:

```sh
./prototype/mani.py git rebase --short
```

The code under `prototype/` is deliberately throwaway. It records the visual
experiment that preceded the Rust implementation and is not a production base.
