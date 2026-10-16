# The MCP server

> **Last updated:** 2026-10-14

`covercraft-cli mcp` speaks the Model Context Protocol over stdio (newline-delimited JSON-RPC).
Configure it in your assistant's MCP settings, or point it at a running server:
`covercraft-cli mcp --server http://127.0.0.1:7979 --token … --user NAME`.

| Tool | What it does |
|---|---|
| `verify_coverage` | The report for comune + via + civico (or `egon`); candidates when more than one civico matches |
| `search_comune`, `search_street`, `list_civici` | Help spelling the address as the tables do |
| `radio_bridges` | The operator's radio bridges |
| `find_cell` | Where a mobile cell is, from MCC, MNC, LAC/TAC and CID/ECI (or eNodeB + sector): the table first, then OpenCelliD's `/cell/get` with the token |
| `history`, `open_history` | Past verifications and their full report |
| `database_settings`, `configure_database`, `test_database` | The database connection (never the password) |
| `get_preferences`, `set_preferences` | Language, logging, radio reach by band, limits, wording rules |

Prompt `check_address` guides an assistant through a verification; resources
`covercraft://report` and `covercraft://bridges` expose the last report and the bridges as text.
