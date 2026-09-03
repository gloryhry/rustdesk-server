# RustDesk API Web

Minimal standalone Vite frontend for the compatible RustDesk HTTP API.

## Screens

- Account login and optional public registration
- Current-user details
- Personal address-book JSON editor
- Administrator user list
- `zh-CN` and `en-US` locale toggle

The frontend uses these API endpoints:

- `POST /api/login`
- `POST /api/register`
- `GET /api/currentUser`
- `GET` and `POST /api/ab`
- `GET /api/admin/user/list`
- `POST /api/logout`

## Development

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
