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
- Typo-tolerant street search and a civici cache across comuni.
- Export of the report (PDF/print page), map tiles for bridges.

## Progress log

| Date | What landed |
|---|---|
| 2026-10-14 | First complete version: core model and interpretation, SQL on MySQL/SQLite with the schema-derived test database, engine with commands and worker, egui interface (address bar with completion, report with radar, history, bridges, settings), MCP server, remote session, desktop/CLI/web apps, Docker, docs |
