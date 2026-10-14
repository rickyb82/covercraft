# CoverCraft roadmap

> **Stage:** 0.1 — the whole pipeline works on the demo city and on an SQLite copy of the real
> schema; the first runs against the production `netmap` database are the next thing to prove.

## Now (0.1 — done)

- Workspace in the craft layout: `core` → `db` → `engine` → `ui-egui` / `mcp` / `remote` → apps.
- Address search in the providers' tables (no EGON API): TIM table of the comune, Open Fiber,
  Fastweb; candidates when ambiguous.
- Report: TIM/FiberCop technologies with speeds and distances, Open Fiber staircases and
  XGS-PON POPs, Fastweb FTTH/FTTS/VULA/GPON, own services, radio bridges by distance and
  bearing with reach per band, mobile cells; raw values kept next to every traffic light;
  configurable wording rules.
- Logging to `requests_log` and history.
- Desktop app, web app (WASM) with per-user sessions on the server, CLI, MCP server (12 tools).
- Docker image and compose for a framework.

## Next

- Run against the real database: tune the wording rules on the real values (Open Fiber
  statuses, Fastweb vendibilità, TIM planning columns), confirm `COORDINATE_BUILDING` and
  `polylines` formats, measure query times on the big TIM tables (indexes on `STRADA_CLIENTE`).
- Line of sight with a terrain model (Tinitaly 10 m DEM): profile, Fresnel clearance, obstruction point.
- Typo-tolerant street search and a civici cache across comuni.
- Export of the report (PDF/print page).

## Progress log
- 2026-10-14: "Mine" in the past verifications lists the session's user (it listed everyone's: the page sent no name).
- 2026-10-14: the geocoder error names the server and the note explains what to check (Nominatim still importing, not running, name not resolving); coordinates by hand remain the fallback.

| Date | What landed |
|---|---|
| 2026-10-14 | Map (OpenStreetMap tiles through `walkers`) in the report and in the bridges screen; 5 GHz link budget for every bridge (FSPL, RX, margin, rate, Fresnel; presets 23 dBm / 23 dBi, editable in Settings → Radio links); Nominatim geocoder as the last resort with a local cache, `docker-compose` service with the Italy extract; coordinates by hand in the report; `?settings=off` hides the settings for non-administrators |
| 2026-10-14 | Local history (`history.jsonl` next to the configuration) instead of writing to the database: a read-only MySQL user is enough, `requests_log` only with `log_to_database`; civici without Open Fiber coordinates or a polyline are placed at the middle of the street's other Open Fiber buildings; the polyline is also looked up without leading zeros in the street code |
| 2026-10-14 | No limits on comuni, streets and civici: statements may return 50 000 rows (the previous 2 000 cut the comuni list at the letter B), suggestion menus scroll, `comune.list`/`street.search`/`civico.list` return everything unless `limit` is given |
| 2026-10-14 | Region → province → comune cascade in the address bar (`istat` names on every comune, `comune.list` filters and `regioni`/`province` listings for the CLI and MCP); suggestion menus stay open while the pointer is on them, so a click on a suggestion lands |
| 2026-10-14 | First complete version: core model and interpretation, SQL on MySQL/SQLite with the schema-derived test database, engine with commands and worker, egui interface (address bar with completion, report with radar, history, bridges, settings), MCP server, remote session, desktop/CLI/web apps, Docker, docs |
