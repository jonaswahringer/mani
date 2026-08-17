# Separate private state from the knowledge base

Mani stores configuration, Custom Guides, and guide-only revision diffs under `$XDG_CONFIG_HOME/mani`, falling back to `~/.config/mani`, so users may sync them as dotfiles. Recent commands and Issue Records live under `$XDG_STATE_HOME/mani`, falling back to `~/.local/state/mani`, preventing sensitive machine-local behavior from entering the shareable Knowledge Base Root.
