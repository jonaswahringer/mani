# Distinguish short and complete stdout output

`--short` prints a Custom Guide's title and introductory content before its first level-two heading; when no guide exists, it prints the safely captured `<command> --help` output rather than a man page. `--print` writes the complete selected document to standard output, defaulting to the Custom Guide when both sources exist. `--custom` and `--official` provide explicit source selection, and `--custom` fails clearly when its guide is missing rather than silently falling back.
