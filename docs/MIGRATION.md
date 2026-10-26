# ScanCraft compatibility

The GUI executable is `scancraft`; diagnostics use `scancraft-cli`. Rust packages
and imports use `scancraft-*` / `scancraft_*`. The original development repository
is preserved separately; no historical branch is included in publication history.

Existing `.raptor`, `.rapcal`, `.orproj`, `.meshproj` and other project/recording
formats retain their magic bytes and schemas. Rebranding does not rewrite user
files. Open older projects explicitly by path; existing capture directories remain
usable in the Save folder field. New capture folders default to Documents/ScanCraft.
No existing directory is moved or deleted automatically.

Scanner sessions acquire both the ScanCraft lock and the former native lock to
coordinate with existing clients. `SCANCRAFT_BRIDGE_LOCK` is preferred;
`OPEN_RAPTOR_BRIDGE_LOCK` remains a compatibility fallback. The research bridge's
existing relative home path is retained. Smoke-test environment variables use the
new SCANCRAFT prefix. Old executable/package aliases are not installed.

Original copyright names and compatibility identifiers are intentionally retained.
Historical research documents may refer to evidence not distributed with this
source snapshot. Such historical measurements are not a new release qualification.
