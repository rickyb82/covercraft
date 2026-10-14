# Deploying CoverCraft in a framework (Emily)

> **Last updated:** 2026-10-14

CoverCraft's server (`covercraft-cli serve`) hosts the web app and an HTTP API for a framework
that puts it behind its own login and HTTPS proxy.

## Docker

```sh
COVERCRAFT_TOKEN=$(openssl rand -hex 24) docker compose up -d
```

The container listens on `127.0.0.1:7979`, serves the web app at `/` and the API under `/api/`.
Put the framework's reverse proxy in front (HTTPS), forwarding the user's name in the
`X-Covercraft-User` header and the service token in `Authorization: Bearer …`; the page passes
them on from `?token=…&user=…` and remembers them in the browser.

## The database

Point it at the `netmap` database once, from the web app's Settings (Database → MySQL) or:

```sh
docker exec -it covercraft covercraft-cli db --kind mysql --host 192.168.10.24 --port 3306 \
  --user netmap --password '…' --database netmap
```

The settings land in `/data/config.json`, the password in `/data/secrets.json` (owner-only).
A read-only MySQL user is enough: verifications are kept in `/data/history.jsonl`, and the
database's `requests_log` is written only if the preference `log_to_database` is on.

## API

- `POST /api/cmd` `{"command": "coverage.verify", "params": {"comune": "CAGLIARI", "via": "via Roma", "civico": "10"}}` → `{"result": {"job": N}}`
- `GET /api/snapshot?since=REV` → the state (report, history, suggestions) and the completed
  jobs since `REV`, or `{"unchanged": true}`.
- `GET /api/health`

The framework can also talk MCP to the server: `covercraft-cli mcp --server https://… --token … --user NAME`.

## Geocoding (Nominatim) and the map

The tables place most civici (Open Fiber's building coordinates, the street polylines, the
other buildings on the street). For the rest, CoverCraft asks a Nominatim server, and only then.
`docker-compose.yml` ships one (`mediagis/nominatim`) with the Italy extract from Geofabrik:
the first start imports it (about 2 GB to download, one to three hours, around 10 GB of RAM),
later starts are quick. Set the preference `geocode_url` to `http://nominatim:8080` (Settings →
Preferences → Geocoder, or `covercraft-cli prefs geocode_url=http://nominatim:8080`). Points
found are remembered in `/data/geocache.json`, as are the ones set by hand in the report.

A report note `Geocoding failed (geocoder http://nominatim:8080: Connection refused …)` means the
server named in `geocode_url` is not answering: Nominatim is still importing (the API only listens
once the import is done), the container is not running, or CoverCraft runs in another Compose
project or network where the name `nominatim` does not resolve. Check with:

```sh
docker compose ps nominatim                # must be "running"
docker compose logs -f nominatim           # the import, then "Starting nominatim" when ready
curl -s http://127.0.0.1:8081/status       # "OK" from the host once the API is up
docker exec covercraft getent hosts nominatim   # the name must resolve from CoverCraft's container
```

Until the server answers, the report shows the note and the civici can still be placed by hand
(the coordinates fields under the map) or with `covercraft-cli verify … --lat … --lon …`.

The map uses OpenStreetMap's tiles by default (`https://tile.openstreetmap.org/{z}/{x}/{y}.png`,
see the OSMF tile usage policy for anything beyond light use); `tiles_url` points it at your
own tile server.

## Radio links

The report estimates every bridge's link with a free-space budget (5.5 GHz, 40 MHz, 23 dBm,
23 dBi on both ends, 1 dB of cable loss per side, 10 dB fade margin, -75 dBm minimum RX by
default; Settings → Radio links). It assumes a clear path: the terrain is not checked yet.

## Settings for administrators only

A framework that puts CoverCraft behind its own login can open the web app with
`?settings=off` for non-administrators: the settings screen and its button are hidden. That
is cosmetic; the framework's proxy must also refuse `db.configure`, `db.test` and
`prefs.set` (apart from `theme` and `language`) for those users.
