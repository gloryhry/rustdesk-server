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
| `API_TOKEN_TTL` | `3600` | Access-token lifetime in seconds. |
| `API_REGISTER_ENABLED` | `0` | Set to `1` to enable public registration. Publicly registered accounts are ordinary users. |
| `API_BOOTSTRAP_ADMIN_USERNAME` | *(unset)* | Optional administrator username used only when the database has no users. Set together with the password. |
| `API_BOOTSTRAP_ADMIN_PASSWORD` | *(unset)* | Optional bootstrap administrator password. Never log or commit this value. |
| `API_PUBLIC_URL` | listener URL | Public URL returned to authenticated Web Clients. Set this explicitly behind a proxy. |
| `API_WEB_ROOT` | `./web/dist` | Directory containing built Web Admin/Web Client assets served by `rustdesk-api`. |
| `API_OAUTH_REDIRECT_URL` | *(unset)* | Exact HTTPS callback URL registered with the OAuth provider, for example `https://api.example.com/api/oidc/callback`. OAuth login remains unavailable until this is set. |
| `API_GITHUB_CLIENT_ID` / `API_GITHUB_CLIENT_SECRET` | *(unset)* | Optional GitHub OAuth application credentials. The secret must remain server-side. |
| `API_GOOGLE_CLIENT_ID` / `API_GOOGLE_CLIENT_SECRET` | *(unset)* | Optional Google OAuth application credentials. The secret must remain server-side. |
| `API_OIDC_CLIENT_ID` / `API_OIDC_CLIENT_SECRET` | *(unset)* | Optional generic OIDC client credentials. |
| `API_OIDC_AUTH_URL` | *(unset)* | Generic OIDC authorization endpoint. |
| `API_OIDC_TOKEN_URL` | *(unset)* | Generic OIDC token endpoint. |
| `API_OIDC_USERINFO_URL` | *(unset)* | Generic OIDC userinfo endpoint. |
| `API_OIDC_ISSUER_URL` / `API_OIDC_JWKS_URL` | *(unset)* | Optional OIDC ID-token validation endpoints. Set both to require signed ID-token verification with issuer, audience, expiry, subject, and nonce checks. |
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
