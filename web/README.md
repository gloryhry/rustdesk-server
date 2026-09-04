# RustDesk API Web

Minimal standalone Vite frontend for the compatible RustDesk HTTP API.

## Screens

- Account login and optional public registration
- Current-user details
- Personal address-book JSON editor
- Address-book tag list, creation/update, and deletion
- Administrator user list
- Administrator session list and revocation
- Administrator OAuth provider status and endpoint view
- Administrator LDAP configuration editor with bind-password redaction
- Authenticated device list
- Authenticated user-group list
- Authenticated device-group list, creation, deletion, and device assignment
- `zh-CN` and `en-US` locale toggle

The frontend uses these API endpoints:

- `GET` and `POST /api/oidc/auth`, `GET /api/oidc/auth-query`, and `GET /api/oidc/callback`
- `GET /api/user/info`
- `POST /api/register`
- `GET /api/currentUser`
- `GET` and `POST /api/ab`
- `GET` and `POST /api/ab/peers`
- `GET` and `POST /api/ab/tags`
- `POST /api/ab/tags/delete`
- `GET /api/admin/user/list`
- `POST /api/admin/user/create`, `/api/admin/user/update`, and `/api/admin/user/delete`
- `GET /api/admin/device/list` and `POST /api/admin/device/delete`
- `GET /api/admin/session/list`
- `POST /api/admin/session/revoke`
- `GET` and `POST /api/admin/ldap/config`
- `GET /api/devices`
- `GET /api/groups` (the GET response includes `memberships`)
- `POST /api/groups/members` and `/api/groups/members/delete`
- `GET` and `POST /api/device-groups` (the GET response includes `memberships`)
- `POST /api/device-groups/delete`
- `POST /api/device-groups/members`
- `POST /api/device-groups/members/delete`
- `POST /api/logout`

After successful login, the frontend automatically requests the public server configuration and the account's personal address book; the profile view displays the public server values without exposing private server or provider secrets.


Requirements: Node.js 18 or newer and a RustDesk API server listening on `127.0.0.1:21114`.

```bash
cd web
npm install
npm run dev
```

Open `http://127.0.0.1:5178`. Vite proxies `/api` to `http://127.0.0.1:21114` by default.

To use another development API server:

```bash
VITE_API_PROXY_TARGET=http://192.0.2.10:21114 npm run dev
```

## Production Build

```bash
cd web
npm install
npm run build
```

The static output is written to `web/dist`. Serve that directory from the same origin as the API, or set the public API base URL while building:

```bash
VITE_API_BASE=https://rustdesk.example.com npm run build
```

When `VITE_API_BASE` is empty, requests use same-origin `/api` paths. Production deployments must use HTTPS because the browser sends the Bearer access token with protected API requests.

The access token is stored in browser `sessionStorage`, so closing the tab ends the local browser session. Server-side logout also calls `POST /api/logout` to revoke the current API session.

LDAP updates are applied only to the running API process and are reloaded from environment variables after restart. The stored bind password is never returned to the browser; leave the password field empty to preserve the current runtime value.
