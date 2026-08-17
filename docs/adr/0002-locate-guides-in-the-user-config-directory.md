# Locate guides in the user config directory

Mani resolves the Knowledge Base Root from `MANI_HOME` when set, otherwise from `$XDG_CONFIG_HOME/mani`, falling back to `~/.config/mani`. This makes Custom Guides portable with a user's dotfiles while retaining an explicit override for alternative or project-specific collections.
