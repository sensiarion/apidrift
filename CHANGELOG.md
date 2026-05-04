Changelog
=========


0.1.6
--------

* **BREAKING:** compact YAML output — short field names (`desc`, `level`, `schemas`, `routes`, `items`, `details`),
  no emojis, no nulls, no empty arrays, no `change_level_class`/`change_key`/`is_*` fields.
  Routes shown inline per change. ~50% size reduction vs 0.1.5.

0.1.5
--------

* add `--format yaml` for AI-agent oriented output (compact: only `stats` + `grouped_changes`, to reduce context
  overflow)

0.1.4
--------

* add parsing error tracing to more explicitly warn
