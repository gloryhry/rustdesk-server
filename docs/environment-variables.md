# Configuration & Environment Variables

This document is the single reference for every option that the open‑source
RustDesk server binaries (`hbbs`, `hbbr`) understand: command‑line flags,
environment variables, and configuration files.

> **TL;DR** — For most people the command‑line flags shown by `hbbs --help` /
> `hbbr --help` are all you need. Environment variables are an alternative way to
> set the same options, plus a handful of extra tuning knobs that have no flag.

---

## How configuration is loaded

Both servers read their configuration from the following sources. For
**`hbbs`** the order of precedence, from highest to lowest, is:

1. **Command‑line flag** (e.g. `-p 21116`, `-k mykey`)
2. **`--config <file>`** — an INI file passed with `-c`/`--config`
3. **`.env`** — an INI file named `.env` in the working directory
4. **Inherited process environment** — variables exported before launch

A value set by a higher source overrides the same value from a lower one. Under
the hood every source is turned into a process environment variable, and the
code then reads that variable — so "flag", "config file" and "env var" are just
three ways to set the same thing.

For **`hbbr`** the precedence is: **flag** (`-b`, `-p`, `-k`) → **`.env`** →
**inherited environment**.

`RUST_LOG` is an exception to these rules. Both binaries initialize logging
before loading `.env` (or `hbbs`'s `--config` file), so `RUST_LOG` must be set
in the inherited process environment.

---

## `hbbs` — ID / rendezvous server

| Variable | CLI flag | Default | Description |
|---|---|---|---|
| `KEY` | `-k`, `--key` | `-` | Public key clients must use, a base64 secret key, or `-` / `_` to load or generate a key pair (`id_ed25519`, `id_ed25519.pub`). `-` and `_` have the same behavior, so explicitly passing `-k _` to `hbbs` is unnecessary. An explicitly empty value disables key validation; see [Keys](#keys-and-encryption). |
| `BIND` | `-b`, `--bind` | all interfaces | **Available since 1.1.17.** Local IPv4 or IPv6 address on which all `hbbs` TCP, UDP, and WebSocket listeners bind. This does not change the addresses advertised to clients. Supported by `--config`, `.env`, and the inherited environment. |
| `PORT` | `-p`, `--port` | `21116` | Main TCP/UDP listening port. `hbbs` also binds `PORT-1` (NAT type test) and `PORT+2` (WebSocket). |
| `RELAY-SERVERS` | `-r`, `--relay-servers` | *(empty)* | Optional relay server override handed to clients, as comma-separated `host` or `host:port` values. Leave empty when `hbbr` uses the same address as `hbbs` and the standard port `21117`; clients derive it automatically. Set this only when the relay uses a different IP/hostname or a non-standard port. |
| `RMEM` | `-M`, `--rmem` | `0` (system default) | UDP receive‑buffer size in bytes. Raise the OS limit first: `sudo sysctl -w net.core.rmem_max=52428800`. |
| *(config file)* | `-c`, `--config` | *(none)* | Path to an extra INI config file (see precedence above). |
| `TEST_HBBS` 🅴 | *(none)* | *(auto)* | UDP self‑test target checked at start‑up. Set to `no` to skip the check (useful behind some NATs/proxies), or to an explicit `host:port`. |
| `ALWAYS_USE_RELAY` 🅴 | *(none)* | `N` | `Y` forces every session through a relay (disables direct/hole‑punched connections). At runtime, send `always-use-relay Y` or `always-use-relay N` to the `hbbs` [loopback console](#runtime-console). |
| `DB_URL` 🅴 | *(none)* | `./db_v2.sqlite3` | Path/URL of the SQLite database file. See [Database](#database). |
| `MAX_DATABASE_CONNECTIONS` 🅴 | *(none)* | `1` | Size of the SQLite connection pool. |

🅴 = set through the inherited process environment.

> `PORT_FOR_API` / `KEY_FOR_API` remain reserved legacy/Pro integration names.
> The optional open-source API added here uses the explicit `API_*` settings below.

---

## `rustdesk-api` — optional HTTP API

`rustdesk-api` is a separate process. It provides compatible account login endpoints without changing `hbbs`/`hbbr` connection authorization. A client that has not logged into the API can still register an ID, rendezvous, punch holes, and use the relay under the existing `KEY` rules.

| Variable | Default | Description |
|---|---|---|
| `API_ENABLED` | `0` | Set to `1` to start the HTTP listener. The API is opt-in so upgrades do not change existing server behavior. |
| `API_BIND` | `127.0.0.1` | Address for the HTTP API listener. Use `0.0.0.0` in a container and publish it through a TLS reverse proxy. |
| `API_PORT` | `21114` | HTTP API port. |
| `API_JWT_SECRET` | *(required when enabled)* | Signing secret of at least 32 bytes. Do not reuse the RustDesk private key or commit this value. |
| `API_ALLOW_INSECURE_LOCAL_HTTP` | `0` | Explicit local HTTP development exception. Requires a loopback `API_BIND` and an HTTP `API_PUBLIC_URL` whose host is localhost or a loopback IP. Default cookies always use Secure, HttpOnly, SameSite=Lax, and Path=/; forwarded headers never change this policy. |
| `API_ALLOWED_ORIGINS` | *(empty)* | Comma-separated exact Web origins (scheme, hostname and port). The origin of API_PUBLIC_URL is automatically allowed. Wildcards, credentials, paths and trailing slashes are rejected. Unapproved Origin headers are rejected even if a Bearer or Cookie is present. CORS credentials use explicit origins and Vary: Origin. |
| `API_COOKIE_CROSS_SITE` | `0` | Explicit cross-site Cookie opt-in: SameSite=None; Secure. Cannot be combined with local insecure HTTP mode. Third-party Cookie policies may still prevent sessions; same-origin reverse proxy deployment avoids that restriction. |
| `API_TOKEN_TTL` | `3600` | Access-token lifetime in seconds. |
| `API_REGISTER_ENABLED` | `0` | Set to `1` to enable public registration. Publicly registered accounts are ordinary users. |
| `API_BOOTSTRAP_ADMIN_USERNAME` | *(unset)* | Optional administrator username used only when the database has no users. Set together with the password. |
| `API_BOOTSTRAP_ADMIN_PASSWORD` | *(unset)* | Optional bootstrap administrator password. Never log or commit this value. |
| `API_PUBLIC_URL` | listener URL | Public URL returned to authenticated Web Clients. Set this explicitly behind a proxy. |
| `API_WEB_ROOT` | `./web/dist` | Directory containing built Web Admin/Web Client assets served by `rustdesk-api`. |
| `API_OAUTH_CONFIG_KEY` | *(required for database providers)* | Independent base64-encoded 32-byte key for authenticated encryption of provider client secrets. Keep it outside the database and separate from the JWT signing key. Missing/wrong keys fail closed when database providers exist. Back up this key with the matching database; changing it requires an explicit re-encryption procedure. |
| `API_OAUTH_REDIRECT_URL` | *(unset)* | Exact HTTPS callback URL registered with the OAuth provider, for example `https://api.example.com/api/oidc/callback`. OAuth login remains unavailable until this is set. Browser callbacks also require the initiating flow cookie; state is single-use and expires after 300 seconds. Native login returns a separate polling code and browser-launch URL; the callback does not log the browser in. Only the initiating code/id/uuid tuple can claim the result once. Pending native flows are limited to 10000 and expire after 300 seconds; a restart invalidates unfinished flows. |
| `API_GITHUB_CLIENT_ID` / `API_GITHUB_CLIENT_SECRET` | *(unset)* | Optional GitHub OAuth2 application credentials. Both credentials are required when enabled; no ID token is required. The secret must remain server-side. |
| `API_GOOGLE_CLIENT_ID` / `API_GOOGLE_CLIENT_SECRET` | *(unset)* | Optional Google OIDC application credentials. Both credentials are required when enabled; issuer is https://accounts.google.com and JWKS is https://www.googleapis.com/oauth2/v3/certs. The secret must remain server-side. |
| `API_OIDC_CLIENT_ID` / `API_OIDC_CLIENT_SECRET` | *(unset)* | Optional generic OIDC client credentials. |
| `API_OIDC_AUTH_URL` | *(unset)* | Generic OIDC authorization endpoint. |
| `API_OIDC_TOKEN_URL` | *(unset)* | Generic OIDC token endpoint. |
| `API_OIDC_USERINFO_URL` | *(unset)* | Generic OIDC userinfo endpoint. |
| `API_OIDC_ISSUER_URL` / `API_OIDC_JWKS_URL` | *(unset)* | Both endpoints are required when generic OIDC is configured. Invalid or partial provider configuration prevents startup with an explicit configuration error; it never falls back to an unvalidated provider. OIDC callbacks require an ID token with a valid signature, issuer, audience, expiry, nonce and subject; userinfo.sub must match. Multiple audiences require a matching azp. |
| `API_OIDC_SCOPE` | `openid email profile` | Generic OIDC scopes requested during authorization. |
| `API_LDAP_ENABLED` | `0` | Enables validation and exposure of LDAP configuration to administrators. Directory bind authentication requires the LDAP protocol adapter and is not enabled by this setting alone. |
| `API_LDAP_URL` | *(unset)* | LDAP endpoint using `ldap://` or `ldaps://`. |
| `API_LDAP_BIND_DN` | *(unset)* | Service account bind DN. |
| `API_LDAP_BIND_PASSWORD` | *(unset)* | Service account password. It is never returned by the administrator API. |
| `API_LDAP_USER_BASE_DN` | *(unset)* | Base DN for user searches. |
| `API_LDAP_USER_FILTER` | `(&(objectClass=person)(uid={username}))` | User search filter; must contain `{username}`. |
| `API_LDAP_USERNAME_ATTRIBUTE` | `uid` | Directory attribute mapped to API username. |
| `API_LDAP_EMAIL_ATTRIBUTE` | `mail` | Directory attribute mapped to API email. |
| `API_LDAP_USE_TLS` | `0` | Marks StartTLS use for the future LDAP adapter; prefer `ldaps://` where supported. |
| `API_LDAP_TIMEOUT` | `5` | LDAP operation timeout in seconds, constrained to `1..=60`. |
| `RUSTDESK_ID_SERVER` | *(empty)* | ID server address returned by `/api/server-config`. |
| `RUSTDESK_RELAY_SERVER` | *(empty)* | Relay server address returned by `/api/server-config`. |
| `RUSTDESK_KEY_FILE` | `id_ed25519.pub` | File containing the public RustDesk server key returned to clients. |
| `RUSTDESK_KEY` | *(unset)* | Explicit public key override. Never put the private key here. |
| `DB_URL` | `./db_v2.sqlite3` | Shared SQLite database used by `hbbs` and `rustdesk-api`. |

Current compatible endpoints include `POST /api/login`, `POST /api/logout`, `GET`/`POST /api/currentUser`, authenticated `GET /api/users`, `/api/peers`, and `/api/device-group/accessible`, `GET /api/login-options`, `GET`/`POST /api/oidc/auth`, `GET /api/oidc/login`, `GET /api/oidc/callback`, `GET /api/oauth/login`, `GET /api/oauth/callback`, registration routes, public `POST /api/sysinfo`/`sysinfo_ver`, authenticated `GET /api/devices`, authenticated `GET`/`POST /api/ab`, authenticated `/api/ab/tags` and `/api/ab/tags/delete`, authenticated `/api/groups` and `/api/groups/delete`, authenticated `/api/device-groups`, `/api/device-groups/delete`, `/api/device-groups/members`, and `/api/device-groups/members/delete` routes, and authenticated `/api/server-config`/`server-config-v2`. OAuth callbacks use a one-time five-minute state value and create or reuse an API account linked to the provider subject. Browser OAuth callbacks (`Accept: text/html`) set an `HttpOnly`, `SameSite=Lax` session cookie and redirect to `/`; JSON clients continue receiving the bearer token response. Administrators can inspect or validate runtime LDAP settings through `GET`/`POST /api/admin/ldap/config`; responses never contain the bind password, and runtime updates are not persisted to disk. Login accepts RustDesk device fields such as `id`, `uuid`, `autoLogin`, and `deviceInfo`, and returns `type`, `access_token`, `user`, and `expires_in`.

User groups now include visible `memberships` in `GET /api/groups`; `POST /api/groups/members` and `/api/groups/members/delete` are scoped to the group owner or an administrator and never grant device access by themselves.


Address-book entries can also be managed individually with authenticated `GET`/`POST /api/ab/peers`, `POST /api/ab/peer`, `POST /api/ab/peer/delete`, and guid-scoped peer/tag aliases; these operations are owner-scoped and do not grant remote-control authorization. Stock-client compatibility aliases include `/api/user/info`, JSON `POST /api/oidc/auth`, `GET /api/oidc/auth-query`, and the OIDC message routes. Administrators can list and delete API devices through `GET /api/admin/device/list` and `POST /api/admin/device/delete`.




```bash
export API_ENABLED=1
export API_JWT_SECRET='replace-with-at-least-32-random-bytes'
export API_REGISTER_ENABLED=1
rustdesk-api

curl -X POST http://127.0.0.1:21114/api/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"correct-horse-battery-staple"}'

TOKEN=$(curl -sS -X POST http://127.0.0.1:21114/api/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"correct-horse-battery-staple","id":"123456789","uuid":"client-uuid","deviceInfo":{"name":"office","os":"Linux","type":"desktop"}}' \
  | sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p')

curl http://127.0.0.1:21114/api/currentUser \
  -H "Authorization: Bearer $TOKEN"
```

Bearer tokens and browser cookies must be protected by HTTPS in production. API login is optional and does not gate remote-control connections.

---

## `hbbr` — relay server

| Variable | CLI flag | Default | Description |
|---|---|---|---|
| `KEY` | `-k`, `--key` | *(empty)* | The empty default intentionally disables relay key validation, avoiding key-pair setup and mismatch failures. To enable relay key validation, use the same non-empty key as `hbbs`; `-` / `_` have the same behavior and load or generate a key pair. An empty key allows clients without a matching key to use the relay, so choose this tradeoff deliberately on an exposed server. |
| `BIND` | `-b`, `--bind` | all interfaces | **Available since 1.1.17.** Local IPv4 or IPv6 address on which the relay TCP and WebSocket listeners bind. Supported by `.env` and the inherited environment; `hbbr` does not support `--config`. |
| `PORT` | `-p`, `--port` | `21117` | Relay listening port. `hbbr` also binds `PORT+2` for WebSocket relay. **Note:** when set via the `PORT` env var (not `-p`), `hbbr` listens on `PORT + 1`, so a shared `PORT=21116` makes `hbbs`=21116 and `hbbr`=21117. |

### Relay bandwidth / QoS

These have no CLI flag and can also be changed through the `hbbr`
[loopback console](#runtime-console) (`tb`, `sb`, `ls`, `dt`, `t`, …; send `h`
for help).

| Variable | Default | Unit | Description |
|---|---|---|---|
| `SINGLE_BANDWIDTH` | `128` | Mb/s | Normal maximum bandwidth for each relay connection. |
| `TOTAL_BANDWIDTH` | `1024` | Mb/s | Aggregate bandwidth cap shared by all relay connections. |
| `LIMIT_SPEED` | `32` | Mb/s | Per-connection cap applied after a connection is downgraded, and to IPs in `blacklist.txt`. |
| `DOWNGRADE_THRESHOLD` | `0.66` | ratio (0–1) | Fraction of `SINGLE_BANDWIDTH` that a connection's lifetime-average throughput must exceed to trigger downgrade. |
| `DOWNGRADE_START_CHECK` | `1800` | seconds | Delay before a connection becomes eligible for the lifetime-average downgrade check. |

Downgrade is decided independently for each connection; it does **not** check
aggregate relay congestion. After `DOWNGRADE_START_CHECK`, a connection is
capped to `LIMIT_SPEED` once its average throughput since it started exceeds
`SINGLE_BANDWIDTH * DOWNGRADE_THRESHOLD`. A lone transfer can therefore be
downgraded even when the relay is otherwise idle. `TOTAL_BANDWIDTH` is a
separate aggregate cap.

These may also be placed in `.env` using the uppercase spellings shown above
(e.g. `SINGLE_BANDWIDTH=256`).

### Blocklists / blacklists (files, not env vars)

`hbbr` reads two optional files from its working directory at start‑up:

* **`blacklist.txt`** — IPs that are **bandwidth‑limited** (one IP per line;
  anything after the first space on a line is ignored).
* **`blocklist.txt`** — IPs that are **refused** outright.

Both can also be edited live through the `hbbr` loopback console (`ba`/`br`,
`Ba`/`Br`).

### Runtime console

The runtime consoles are TCP command transports built into the services; they
are not `rustdesk-utils` commands or interactive standard-input consoles. A
connection from a loopback address is treated as a single console command:

```bash
# hbbs: toggle forced relay on PORT-1 (21115 by default)
printf 'always-use-relay Y' | nc 127.0.0.1 21115

# hbbr: list commands on its relay PORT (21117 by default)
printf 'h' | nc 127.0.0.1 21117
```

Use the corresponding configured ports if you changed `PORT`.

---

## Database

At runtime the database location comes from **`DB_URL`** (default
`./db_v2.sqlite3`). If unset, `hbbs` creates the SQLite file in its working
directory.

> **Do not confuse `DB_URL` with `DATABASE_URL`.** The `DATABASE_URL` entry in
> the repository's `.env` is used **only at compile time** by `sqlx` to check SQL
> queries; it is **not** read by the running server. Setting `DATABASE_URL` on a
> running server has no effect — use `DB_URL`.

---

## Logging

Both binaries use `flexi_logger`, which honours the standard **`RUST_LOG`**
environment variable (default level `info`). Set it in the process environment
before launching the binary. A value in `.env` or `hbbs`'s `--config` file is
loaded too late and has no effect on logging.

```bash
RUST_LOG=debug hbbs
```

---

## Keys and encryption

The `KEY` / `-k` value can be:

* a **public key** string — clients must present the matching key;
* a **base64‑encoded 64‑byte secret key** — the server derives the public key
  from it;
* **`-` or `_`** — the server loads a key pair from the working directory or
  generates one on first start, writing `id_ed25519` (private) and
  `id_ed25519.pub` (public);
* **empty** — key validation is disabled. `hbbs` still loads or generates key
  files for signing but deliberately leaves its active validation key empty;
  `hbbr` neither loads nor generates a key. Both services then accept clients
  without validating a key. This is the intentional `hbbr` default; use a
  non-empty value when relay key validation is required.

`hbbs` defaults to `-`, so it already loads or generates a key pair without an
explicit `-k _`. `hbbr` intentionally defaults to an empty key to avoid
key-pair setup and mismatch failures. Leave it empty for the default mode
without key validation. To enable relay key validation, give it the same
non-empty key as `hbbs`; both services can reuse key material from a shared
working directory. The `_` value is not a stricter mode than `-` in the current
implementation.

To supply your own key pair, place `id_ed25519` and `id_ed25519.pub` in the
process's **current working directory** before first start. That directory may
differ from the directory containing the executable. For the supervisor Docker
image, the working directory is `/data`.

---

## Docker image variables

The supervisor image (`rustdesk/rustdesk-server-s6`) starts both binaries with
s6 and adds a few convenience variables handled by its service scripts, **not**
by `hbbs`/`hbbr` directly:

| Variable | Default | Description |
|---|---|---|
| `RELAY` | `relay.example.com` | Passed to `hbbs` as `-r $RELAY` (your public address). |
| `ENCRYPTED_ONLY` | `0` | `1` adds `-k _` to both servers. This is redundant for `hbbs`, whose default is `-`, and opts `hbbr` into key validation instead of its intentional empty default. |
| `KEY_PUB` | *(unset)* | If set, written to `/data/id_ed25519.pub` on first start. |
| `KEY_PRIV` | *(unset)* | If set, written to `/data/id_ed25519` on first start. Provide **both** `KEY_PUB` and `KEY_PRIV`, or neither. |

Any variable from the tables above can also be passed straight through the
container's environment (e.g. `-e ALWAYS_USE_RELAY=Y`, `-e RUST_LOG=debug`).

The classic scratch image (`rustdesk/rustdesk-server`) contains only the
binaries and does **not** implement `RELAY`, `ENCRYPTED_ONLY`, `KEY_PUB`, or
`KEY_PRIV`; those variables are ignored by that image.

---

## Examples

### Command line — non-standard ports

```bash
# Tell clients where the relay listens because it is not using port 21117.
hbbs -p 22116 -r rustdesk.example.com:22117
hbbr -p 22117
```

### `.env` file (working directory)

```ini
# Non-standard ports shared by both binaries; hbbr listens on PORT+1.
relay-servers=rustdesk.example.com:22117
PORT=22116
```

### docker-compose

```yaml
services:
  rustdesk-server:
    image: rustdesk/rustdesk-server-s6:latest
    environment:
      - RELAY=rustdesk.example.com:21117
      - ALWAYS_USE_RELAY=Y
      - RUST_LOG=info
      - SINGLE_BANDWIDTH=256
    ports:
      - "21115:21115"
      - "21116:21116"
      - "21116:21116/udp"
      - "21117:21117"
      - "21118:21118"
      - "21119:21119"
    volumes: ["./data:/data"]
    restart: unless-stopped
```

### systemd

```ini
[Service]
Environment=ALWAYS_USE_RELAY=Y
Environment=RUST_LOG=info
ExecStart=/usr/bin/hbbs
```

---

## Port reference

| Port | Proto | Server | Purpose |
|---|---|---|---|
| 21115 | TCP | hbbs | NAT type test (`PORT-1`) |
| 21116 | TCP + UDP | hbbs | ID registration / rendezvous / hole punching (`PORT`) |
| 21117 | TCP | hbbr | Relay (`hbbr PORT`) |
| 21118 | TCP | hbbs | WebSocket rendezvous (`PORT+2`) |
| 21119 | TCP | hbbr | WebSocket relay (`hbbr PORT+2`) |

Ports 21118/21119 are only needed for the web client; you can omit them
otherwise.

### Persisted OAuth provider management

Administrators can list/create providers with `GET`/`POST /api/admin/oauth/providers`,
edit with `POST /api/admin/oauth/providers/update`, change enabled state with
`POST /api/admin/oauth/providers/toggle`, and delete with
`POST /api/admin/oauth/providers/delete`. Edits, toggles and deletes use the stable
`id` from the list. Provider requests include `kind` (`oauth2` or `oidc`), `name`,
`client_id`, the authorization/token/userinfo endpoints, scopes and `enabled`.
OIDC also requires issuer/JWKS URLs and the `openid` scope. Creation requires a
secret; an empty or whitespace-only edit preserves the existing secret.
Responses expose only `secret_configured`, never the secret or ciphertext.
Environment providers are marked `source=environment, read_only=true` and cannot
be modified through these APIs.

Migration version 1 reserves historical provider names and stores stable IDs and
identity namespaces. SQLite transactions serialize migrations; failed lock
upgrades retry the whole transaction. Provider names and identity authority
(type, client ID and identity endpoints) are immutable, including after deletion.
Use a new name for a new authority. Secret/scope edits preserve account links.
Successful mutations replace the runtime configuration atomically and invalidate
older pending and unclaimed native authorizations; disabling also blocks new
flows immediately. A restart preserves configurations but invalidates unfinished
OAuth flows. Run only one API process per registry database: runtime replacement
is process-local; hbbs/hbbr may share the database.

Before upgrading a real database, stop writers and create a consistent SQLite
backup with its encrypted-configuration key; rehearse migration on a copy. A
rollback must restore the matching program version, database backup and key.
Neither tests nor local commits here migrate a production database.

For browser acceptance, build `rustdesk-api`, run `npm ci && npm run build` in
`web`, then set `RUSTDESK_API_BINARY` to its absolute path and run
`npm run test:browser`. The harness uses a temporary database, random loopback
port and temporary keys. `PLAYWRIGHT_CHROME` optionally selects a local Chrome
executable (default `/usr/bin/google-chrome`); it does not download a browser.

### Web origins and browser session protection

Web login and refresh use the HttpOnly Cookie; JavaScript no longer caches Bearer
tokens in localStorage or sessionStorage. Fetch requests always include
credentials. After login or reload, `GET /api/session/csrf` returns the authenticated
user and a session-bound `csrf_token` with `Cache-Control: no-store`. Cookie-authenticated
modifications must include both an approved Origin and `X-CSRF-Token`; the token
from another session is rejected. Read-only POST compatibility aliases for
currentUser/user info/server configuration do not require CSRF. JSON login,
registration and native OAuth initiation remain available to clients without a
browser session. Bearer-authenticated native mutations do not require CSRF;
an invalid or malformed Authorization header never falls back to Cookie auth.

For same-site Web/API origins such as `https://web.example.com` and
`https://api.example.com`, set API_PUBLIC_URL to the API URL,
API_ALLOWED_ORIGINS to the exact Web origin and build Web with
VITE_API_BASE pointing to the API. Default SameSite=Lax remains appropriate.
For unrelated sites, additionally opt into API_COOKIE_CROSS_SITE=1 and HTTPS.
Browser third-party Cookie restrictions can still reject login: the Web UI reports
that the Cookie session is unavailable. Prefer serving Web and /api through the
same HTTPS reverse proxy when this occurs. CORS does not bypass browser Cookie
policies. A failed logout keeps the error visible rather than claiming server
session revocation succeeded.

### Unsigned device reports and explicit ownership

Official 1.4.9 sends `POST /api/sysinfo` and `/api/heartbeat` without a Bearer.
Both require `id` and a base64 UUID that decodes to the UUID of a registered hbbs
Peer. Unknown IDs or mismatched UUIDs return the exact text `ID_NOT_FOUND`.
Malformed identities fail with 400; only a committed sysinfo report returns
`SYSINFO_UPDATED`. Heartbeats return a `sysinfo` marker when a matching sysinfo
report is absent, including after the registered key/UUID changes.
`/api/sysinfo_ver` returns `unsigned-report-v1` so existing clients refresh their
old report cache. The full JSON is retained as untrusted telemetry, including
unknown fields; username, UUID and any claimed owner/status/key cannot prove
ownership or change the account association, public key, management status or
observed online time.

Each report body is limited to 64 KiB. Sysinfo and heartbeat each allow one write
per matching Peer every 5 seconds; concurrent duplicates receive 429. At most
10000 report rows are stored; new rows above that limit fail with 503 rather than
claim success. There is no unbounded in-memory report cache. Storage failures
return 500 and roll back. API reports do not refresh trusted registration time.

Administrators inspect `GET /api/admin/device/registry`, which returns up to 100
registered Peers in stable ID order; use `?peer_id=...` to inspect a specific ID.
It separates `untrusted_sysinfo`/`untrusted_heartbeat` from `pk_fingerprint`, owner
and hbbs observations. In the Web device page, select the account and independently
verify the public key from the controlled device over an existing trusted channel
before typing its `sha256:<hex>` fingerprint. Copying a report's username/UUID is
not such verification. `POST /api/admin/device/bind` takes `peer_id`, `user_id`
and `pk_fingerprint`; it rechecks the currently registered key transactionally.
`POST /api/admin/device/unbind` takes the stable device `id`. Bind, unbind and
administrator deletion store an audit record in the same transaction; if audit
storage fails, ownership changes roll back. Account transfers retain the stable
internal device ID and reject conflicting verified records.

Migration version 2 retains historical API devices and group membership data,
marking old associations pending verification. Ordinary users cannot list or add
unverified devices to their groups. Existing links whose registered key/UUID has
changed are also hidden until an administrator verifies them again. Reports alone
cannot upgrade these links. No real database is migrated by the local test suite.

hbbs observes successful registrations and uses a bounded queue of 4096 events;
the background worker persists at most 256 per transaction, with three bounded
retries on storage failure. A full queue never blocks the network loop. Dropped
or failed observations leave API online state conservative. Registration records
match the Peer GUID, UUID and key, preserve the newest observation and survive
API restarts. API online status expires after the existing hbbs timeout of
30000 milliseconds. This means hbbs observed a registration under its existing
protocol rules; it is separate from API ownership verification and remote-control
or relay authorization. Build and deploy the updated hbbs together with the API.

### Native device list and management identifiers

`GET /api/peers` now uses a separate RustDesk 1.4.9 PeerPayload DTO: `id` is the
actual registered RustDesk ID, `info` is a JSON object with string
`username`, `os` and `device_name`, and `user`/`user_name` refer to the verified
account association. Registered reports provide presentation fields only;
legacy JSON extension fields are retained. `status` remains the management
state, while `online` and `registered_at_ms` report hbbs observations separately.
Only currently verified associations are returned; administrators inspect pending
records through management routes. Device-group names respect the requesting
account's existing group scope.

`GET /api/devices` and `/api/admin/device/list` retain stable internal device IDs
for management deletion and group membership. Do not pass a native PeerPayload
`id` to these management operations. A damaged legacy info document does not
break the native object shape: available name/OS fields provide a safe fallback
and `info_error` explicitly reports the corruption. Reads never rewrite or discard
the original stored information, which remains available to administrators.

### Official list pagination (RustDesk 1.4.9)

`GET /api/users`, `/api/peers`, and `/api/device-group/accessible` return
root `total` and `data`. `current` defaults to 1; `pageSize` defaults to 100
and must be between 1 and 100. Invalid or overflowing parameters return 400;
pages past the end return empty data with the filtered total. Sorting is stable
by name/ID (users and groups) or RustDesk ID/internal ID (peers).
Authorization is applied before counting, and `accessible` never broadens scope.
Users/peers accept management `status=0|1`; groups have no status filter.
An optional `name` filter matches literal substrings in account name, group name,
or Peer ID/managed device name. `%` and `_` are not wildcards.
The Web group selector reads all account pages; management-specific list routes
retain their existing complete responses.

`/api/users.data[].id` is the stable API account ID for both administrators and
ordinary users. The latter still see only themselves. Web group membership
uses this ID, and usernames are never accepted as substitute member IDs.
