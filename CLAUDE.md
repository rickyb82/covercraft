# CoverCraft — instructions for agents

CoverCraft is an open-source, Rust-native tool that checks the connectivity available at an
Italian address from the operator's own coverage database (TIM/FiberCop, Open Fiber, Fastweb,
the operator's network, radio bridges, mobile cells). It is part of the *craft* family
(`mailcraft`, `ideacraft`, …) and follows the same conventions.

## Never crash
- No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` or `unsafe` in
  non-test code (crates carry `#![deny(clippy::unwrap_used, …)]`, `unsafe_code = "forbid"`).
- Errors are `Result` through each crate's error type (`core::Error`, `engine::Error`).
- Input is hostile: database cells, command and MCP parameters, config files. Every cell is
  text until `core` interprets it; `get()` over indexing; bounded lists (`core::limits`);
  table names from the database go through `db::sql::table_ident` before any statement.
- The worker catches a panicking query and resets the connection; `Session::execute` catches a
  panicking command.
- Every crash fix lands with a regression test. `every_command_survives_empty_and_hostile_params`
  guards the engine; the SQL runs in tests on an SQLite copy of the real schema (`crates/db/src/demo.rs`).

## Data and clean-room
- `schema/netmap.sql` is the operator's own schema (a MySQL dump, no data). Never commit rows
  from the real database, credentials or customer addresses; the demo city in
  `crates/core/src/mock.rs` is made up.
- The meaning of the providers' cells (`SI`, `VENDIBILE`, Open Fiber state codes…) is read by
  `core::status` from built-in wording plus configurable rules; when a real value is unknown,
  keep it visible as the raw value instead of guessing.
- No vendor artwork: icons are strokes in `crates/ui-egui/src/icons.rs`.

## Architecture
- Layers (checked by `cargo xtask layers`): `core` (L0) → `db` (L1) → `engine` (L5) →
  `ui-egui` / `mcp` / `remote` (L6) → `apps/*` (L7). Nothing below L6 depends on egui; `engine`
  depends on `core` only (the database is injected as a `SourceFactory`); only apps use
  `rfd`/`eframe`. The UI talks to a `CoverSession` (local `Session` or `RemoteSession`).
- **Everything is a command** (`crates/engine/src/cmd.rs`): the UI, the CLI and the MCP server
  call the same commands; a new user-visible feature is a command first.
- **The UI is thin**: it reads `State` and acts through `CoverApp::run`; colours from
  `theme::Tokens`, icons from `icons.rs`. Programmatic calls never open dialogs.

## Working
- Quality gate before every commit: `cargo xtask ci` (fmt, clippy -D warnings, tests, layers,
  wasm). **Never break wasm**: `apps/covercraft-web` must build for `wasm32-unknown-unknown`.
- Interface strings are English in code and translated in `crates/ui-egui/src/i18n/` (add
  every new label to `i18n::LABELS` and the Italian table).
- For UI work, look at the result:
  `cargo run -p covercraft-ui-egui --example snapshot -- out.png --lang it [--pick|--history|--radio|--settings]`.
- Try it end to end: `cargo run -p covercraft-cli -- verify cagliari "via roma" 10 --demo --lang it`.
- Update `ROADMAP.md` (progress log) and `docs/` when work lands.
