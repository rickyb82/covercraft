# Architecture

> **Last updated:** 2026-10-14

| Crate | Layer | What it holds |
|---|---|---|
| `crates/core` (`covercraft-core`) | L0 | Addresses as typed and as the tables spell them (`address.rs`); coordinates, distances, bearings and street polylines (`geo.rs`); the meaning of coverage cells, speeds and distances (`status.rs`); rows and the report (`model.rs`); the interpretation of TIM's columns, Open Fiber's state codes, Fastweb's tables and the operator's own services, radio hits and cell hits (`report.rs`); configuration, preferences and secrets (`config.rs`); the `CoverageSource` trait (`source.rs`) and the in-memory demo city (`mock.rs`). No database, wasm-safe. |
| `crates/db` (`covercraft-db`) | L1 | `SqlSource`: the statements, written once for MySQL (`mysql` crate, rustls) and SQLite (`rusqlite`); `demo.rs` turns `schema/netmap.sql` into SQLite DDL and fills it with the demo city, so tests run the real SQL on the real column names. |
| `crates/engine` (`covercraft-engine`) | L5 | `Session`: configuration, state, the command table (`cmd.rs`), the query worker (`worker.rs`: one source per session, jobs → events, panics caught, connection reset on errors), snapshots for the browser, the report in words (`text.rs`). |
| `crates/ui-egui` (`covercraft-ui-egui`) | L6 | `CoverApp`: address bar with comune/street/civico completion, the report (provider cards, Open Fiber staircases, radar of bridges, cells), history, bridges, settings; i18n, theme, icons drawn in code. The `snapshot` example renders it offscreen. |
| `crates/mcp` (`covercraft-mcp`) | L6 | The MCP server over stdio: 12 tools, 1 prompt, 2 resources, each tool an engine command. |
| `crates/remote` (`covercraft-remote`) | L6 | `RemoteSession`: a `CoverSession` mirroring a `covercraft-cli serve` server over HTTP (native and wasm). |
| `apps/covercraft` | L7 | The desktop app (eframe). |
| `apps/covercraft-cli` | L7 | The CLI, `mcp` and `serve` (HTTP API, one session per user, the web app's files). |
| `apps/covercraft-web` | L7 | The web app (wasm32, trunk): the same `CoverApp` on a `RemoteSession`. |
| `xtask` | — | `cargo xtask ci`, `layers`, `wasm`. |

## Commands and jobs

Every action is a `Command` in `crates/engine/src/cmd.rs`. Local commands (`db.get`,
`prefs.*`, `comune.list` once loaded, `history.open`, `report.get`, `session.user`,
`state.get`) answer at once; the others queue a job on the worker and answer `{"job": id}`.
The UI calls `Session::pump` every frame; the CLI and MCP call `run_blocking`.

| Command | Job | Answer |
|---|---|---|
| `db.configure` | reconnects with the saved settings | `{ok, info, database}` |
| `db.test` | pings | same |
| `comune.list` | reads `comuni` (+ `istat`) | `{comuni}` |
| `street.search` | streets of a comune containing the words | `{streets}` |
| `civico.list` | civici of a street (EGON street code) | `{civici}` |
| `address.resolve` | civici matching comune + via + civico | `{candidates}` |
| `coverage.verify` | resolves (or takes `egon`), gathers every table, builds the report, logs it | the report, or `{candidates, ambiguous: true}` |
| `radio.list` | `radio_table` | `{bridges}` |
| `history.list` | `requests_log` newest first | `{history}` |

The report JSON (`report_json`) is the `Report` structure plus `summary`, `summary_it`, `best`
and `text`; the same JSON is written to `requests_log.answer_json`, so `history.open` rebuilds
the report without querying again.

## The address search

The comune names its TIM table (`comuni.table_name`, validated against a strict identifier
pattern before it enters a statement). A typed street becomes a `LIKE` pattern with every word
in order (`%DEI%MILLE%`, the particle `VIA`/`PIAZZA` dropped); the civico is compared without
spaces and in upper case (`10/A`). Matches come from the TIM table first, then `openfiber`,
then `ftth_fw`/`ftts_fw`/`vula_fw`, deduplicated by EGON code. One match verifies at once; more
than one come back as candidates for the person (or the assistant) to pick.

## Interpretation

`core::status::interpret` reads a cell with the configured rules first (`VALUE` or `PREFIX*` →
status), then built-in wording: negatives (`NO`, `NON VENDIBILE`, `ASSENTE`…), planned
(`PIANIFICATO`, `PREVISTO`, a date), positives (`SI`, `COPERTO`, `VENDIBILE`, `ATTIVO`…).
Unknown wording stays `Unknown` and the raw value is always shown, so nothing is guessed in
silence. TIM's hundred columns map to technologies in `report::TIM_TECHS`; Open Fiber's
numeric states go through `openfiber_status` (by value and cluster).

## Multi-user server

`covercraft-cli serve` keeps one `Session` per user named by `X-Covercraft-User` (or `?user=`);
all share the configuration directory (`--home`, `/data` in Docker). A service token
(`--token`, `COVERCRAFT_TOKEN`) is required beyond localhost. Changing the database from one
session saves the files; other sessions reconnect on their next start.
