# CoverCraft

**Verifica di copertura della connettività a un indirizzo, dai tuoi dati.** CoverCraft legge
il database di copertura che già possiedi (le tabelle TIM/FiberCop per comune, Open Fiber,
Fastweb, la tua rete, i tuoi ponti radio, le celle mobili) e, dato comune, via e civico,
risponde con tutte le opzioni disponibili: fibra FTTH, FTTC, rame, fibra dedicata, Open Fiber
scala per scala, i ponti radio raggiungibili con distanza e direzione, le celle vicine.

Scritto in Rust, senza chiamate a servizi esterni: l'indirizzo viene cercato direttamente
nelle tabelle (niente API EGON), il codice EGON del civico esce da lì e con quello si leggono
le coperture. Desktop (macOS, Windows, Linux), web app (WASM) servita dal tuo Docker, CLI e
server MCP per gli assistenti, come [MailCraft](https://github.com/rickyb82/mailcraft).

## Provare subito

```sh
cargo run -p covercraft-cli -- verify cagliari "via roma" 10 --demo --lang it
cargo run -p covercraft -- --demo                       # l'app desktop sulla città di prova
cargo run -p covercraft-cli -- serve --demo --open      # la web app (prima: trunk build in apps/covercraft-web)
```

`--demo` usa una piccola città inventata; con il database vero:

```sh
covercraft-cli db --kind mysql --host 192.168.10.24 --port 3306 --user netmap --password '…' --database netmap
covercraft-cli verify "Quartu Sant'Elena" "via Marconi" 12
```

La password finisce in un file leggibile solo dall'utente (`secrets.json`), mai nei log né
nelle risposte.

## Come funziona una verifica

1. **Comune** → la tabella TIM di quel comune (`comuni.table_name`).
2. **Via e civico** → ricerca nella tabella TIM, poi in `openfiber` e nelle tabelle Fastweb: ne
   esce il codice EGON del civico. Se corrispondono più civici, si sceglie dall'elenco.
3. **Coperture** per codice EGON: la riga TIM (FTTH, XGS-PON, FTTC, EVDSL, SLU, ADSL, SHDSL,
   ULL, WLR, circuiti in fibra, con velocità e distanze), le scale Open Fiber con gli stati
   decodificati da `openfiber_status` e i POP XGS-PON, `ftth_fw`/`ftts_fw`/`vula_fw`/`gpon_fw`
   di Fastweb, `sirius_netmap` (la tua rete).
4. **Coordinate** del civico da `COORDINATE_BUILDING` di Open Fiber, altrimenti dalla
   polilinea della via (`sirius_egon_polylines`), altrimenti dagli altri edifici Open Fiber della
   via, altrimenti da un Nominatim tuo (`docker-compose.yml` ne include uno con l'Italia); si
   possono anche scrivere a mano. Da lì **mappa** (OpenStreetMap), distanza, direzione e
   **bilancio del collegamento** verso ogni ponte radio (`radio_table`, 5 GHz / 23 dBm / 23 dBi
   preimpostati e modificabili: potenza ricevuta, margine, velocità stimata, zona di Fresnel) e
   le celle di `cell_db` (un'esportazione OpenCelliD, da rinfrescare con `covercraft-cli cells
   import`) più, con un token OpenCelliD, le celle in tempo reale intorno al punto (vedi
   `docs/deploy.md`).
5. Ogni stato è un semaforo (coperto / pianificato / non coperto / non noto) **con accanto il
   valore originale della cella**; le parole che il semaforo non conosce si insegnano nelle
   impostazioni (regole `VALORE=stato`).
6. La verifica finisce nello storico locale (`history.jsonl` accanto alla configurazione) con
   l'utente, e si riapre senza interrogare di nuovo; l'utente del database può essere in sola
   lettura (la scrittura in `requests_log` è un'opzione, spenta).

## Struttura

`crates/core` (modello, geometria, interpretazione) → `crates/db` (SQL su MySQL o SQLite,
con un database di prova costruito dallo schema vero per i test) → `crates/engine` (sessione,
comandi, worker) → `crates/ui-egui`, `crates/mcp`, `crates/remote` → `apps/covercraft`
(desktop), `apps/covercraft-cli` (CLI, `serve`, `mcp`), `apps/covercraft-web` (browser).
Dettagli in [`docs/architecture.md`](docs/architecture.md); installazione dentro Emily in
[`docs/deploy.md`](docs/deploy.md); strumenti MCP in [`docs/mcp.md`](docs/mcp.md).

## Sviluppo

`cargo xtask ci` prima di ogni commit (fmt, clippy, test, livelli, wasm). Per guardare
l'interfaccia senza finestra: `cargo run -p covercraft-ui-egui --example snapshot -- out.png --lang it`.

Licenza MIT o Apache-2.0, a scelta.
