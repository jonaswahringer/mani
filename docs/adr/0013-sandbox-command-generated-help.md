# Sandbox command-generated help

Official Mode prefers the installed man page for the full Command Path. When none exists, Mani executes the exact path with only `--help` appended, without shell interpolation, with stdin closed, stdout and stderr captured, and a short timeout. The result is labeled as command-generated help so users can distinguish it from an installed man page.
