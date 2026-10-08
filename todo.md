# Todo

- C1. SecretStore integration: tokens, passwords, DB key; Android via host callback
- C11. Threat model and trust boundaries document (core, AI, MCP)
- F4. mailune-config: typed TOML, layering, committed JSON Schema
- F5. Telemetry: tracing, rotating logs, secret redaction, optional OTLP (off by default)
- F10. Gettext catalogs for core-originated strings
- R1. Core CI: pyrlyn/ci ci-rust.yml matrix + changes.yml + pipeline.yml
- R17. Brand entry (pyrlyn/brand brands/mailune) and landing docs/site.md
- X1. Layered TOML config loader in packages/crates (extend config-schema or add layered-config)
- X2. Extract keychain secret store (env → keyring, no inline secrets) — secret-store
- X3. Extract telemetry setup with redaction — telemetry-setup
- X4. Extract gettext catalog loader — gettext-catalog
- X9. Extract ABI drift test helper (cbindgen + csbindgen regenerate & diff, BLESS env) — abi-drift
- X10. Consume text-sanitize from packages/crates (aulo S1 T1.11, in flight)
- X11. Extract SQLite change feed (PRAGMA data_version poller) — sqlite-change-feed
