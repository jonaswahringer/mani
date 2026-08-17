# Use TOML for user configuration

Mani uses `config.toml` at the Knowledge Base Root for user-editable settings, including the Generator Command. TOML keeps command argument arrays readable, supports comments, and fits dotfiles workflows better than a comment-free JSON document. Invalid configuration stops normal startup with the exact file, line, key or value, and a suggested correction rather than silently applying defaults; `mani config check` validates explicitly, while `--ignore-config` preserves access to core help during recovery.
