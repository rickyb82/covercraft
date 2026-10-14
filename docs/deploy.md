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
The MySQL user needs `SELECT` on the schema and `INSERT` on `requests_log`.

## API

- `POST /api/cmd` `{"command": "coverage.verify", "params": {"comune": "CAGLIARI", "via": "via Roma", "civico": "10"}}` → `{"result": {"job": N}}`
- `GET /api/snapshot?since=REV` → the state (report, history, suggestions) and the completed
  jobs since `REV`, or `{"unchanged": true}`.
- `GET /api/health`

The framework can also talk MCP to the server: `covercraft-cli mcp --server https://… --token … --user NAME`.
