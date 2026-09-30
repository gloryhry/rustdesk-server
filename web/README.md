# RustDesk API Web Admin / Web Client

这是 RustDesk 独立 HTTP API 的 Web 管理后台和浏览器客户端，使用 Vite 构建为静态资源。它可以在开发时由 Vite 独立运行，也可以由 `rustdesk-api` 通过 `API_WEB_ROOT` 直接提供。

## 当前功能

- 用户登录、公开注册和可用 OAuth/OIDC provider 登录。
- HttpOnly cookie session 恢复、注销和当前用户资料。
- 服务器公开配置展示。
- 个人地址簿的 raw JSON snapshot 编辑。
- 地址簿 peer 的结构化添加、编辑、删除。
- 地址簿标签创建、颜色设置和删除。
- 普通用户和管理员用户管理视图。
- 用户组创建、删除、成员添加和成员移除。
- 设备列表和状态展示。
- 管理员设备删除。
- 设备组创建、删除、设备分配和移除。
- 管理员 session 列表和 session 撤销。
- 管理员 OAuth provider 状态查看。
- 管理员 LDAP 配置编辑，bind password 不返回给浏览器。
- `zh-CN` 和 `en-US` locale 切换。

## API 路由

前端使用同源 `/api` 路径，或使用构建时的 `VITE_API_BASE` 指向独立 API。主要请求包括：

- `GET`/`POST /api/oidc/auth`、`GET /api/oidc/auth-query`、`GET /api/oidc/callback`
- `GET /api/user/info`、`GET /api/currentUser`、`POST /api/register`
- `GET`/`POST /api/ab`、`GET`/`POST /api/ab/peers`
- `POST /api/ab/peer`、`DELETE /api/ab/peer/:guid`
- `GET`/`POST /api/ab/tags`、`POST /api/ab/tags/delete`
- `GET /api/admin/user/list`、`POST /api/admin/user/create`
- `POST /api/admin/user/update`、`/api/admin/user/delete`
- `GET /api/admin/device/list`、`POST /api/admin/device/delete`
- `GET /api/admin/session/list`、`POST /api/admin/session/revoke`
- `GET`/`POST /api/admin/ldap/config`
- `GET /api/devices`、`GET`/`POST /api/groups`
- `POST /api/groups/members`、`/api/groups/members/delete`
- `GET`/`POST /api/device-groups`
- `POST /api/device-groups/delete`、`/api/device-groups/members`、`/api/device-groups/members/delete`
- `POST /api/logout`

管理员操作由服务端鉴权，前端隐藏管理控件不等于权限控制。生产环境必须依赖 API 服务端的管理员校验。

## 环境要求

- Node.js 18 或更新版本。
- npm。
- 一个运行在 `127.0.0.1:21114` 的 RustDesk API 服务，或通过环境变量指定其他地址。

## 开发模式

在仓库根目录执行：

```bash
cd web
npm ci
npm run dev
```

打开 <http://127.0.0.1:5178>。Vite 默认将 `/api` 代理到 `http://127.0.0.1:21114`。

指定其他开发 API：

```bash
VITE_API_PROXY_TARGET=http://192.0.2.10:21114 npm run dev
```

开发时 API 需要先启用，例如：

```bash
API_ENABLED=1 \
API_BIND=127.0.0.1 \
API_PORT=21114 \
API_JWT_SECRET='replace-with-at-least-32-random-bytes' \
DB_URL="$PWD/data/db.sqlite3" \
./target/release/rustdesk-api
```

## 生产构建

```bash
cd web
npm ci
npm run build
```

构建结果写入 `web/dist`。可由 Nginx/Caddy 等 HTTPS 反向代理提供，并把 `/api` 代理到 API；也可以由 API 进程直接提供：

```bash
API_WEB_ROOT=/absolute/path/to/rustdesk-server/web/dist \
./target/release/rustdesk-api
```

跨域或独立前端构建时设置 API base：

```bash
VITE_API_BASE=https://rustdesk.example.com npm run build
```

当 `VITE_API_BASE` 为空时，前端使用同源 `/api`。生产环境必须使用 HTTPS。Web 统一使用 HttpOnly Cookie 会话；请求携带 credentials，修改请求附带会话 CSRF token，刷新通过 `/api/session/csrf` 恢复，旧 Bearer 缓存会被清除。

## Session 行为

成功登录后，前端优先使用服务端设置的 `rustdesk_api_token` HttpOnly cookie。cookie session 可以在浏览器刷新后恢复；退出登录会调用 `POST /api/logout`，由服务端撤销 session 并清除 cookie。API/native client 也可以读取 JSON access token 并使用 `Authorization: Bearer <token>`。

前端不会读取 HttpOnly cookie 的内容。启动时通过 `/api/currentUser` 探测 cookie session；失败后回到登录页。不要通过修改前端状态绕过服务端授权。

## OAuth/OIDC

OAuth 登录入口通过 `/api/oidc/auth` 启动，callback URL 由 API 服务端配置。服务端负责 state、PKCE 和 nonce；浏览器 callback 成功后会设置 cookie 并回到首页，API/native caller 则可以接收 JSON token。

管理员 provider 状态来自 `/api/admin/oauth/providers`。完整的 issuer、audience、JWKS 和 endpoint 配置请查看仓库根目录的 [`docs/environment-variables.md`](../docs/environment-variables.md)。

## LDAP

LDAP 配置页面用于查看和更新 API 进程内的 LDAP 配置，并且不会回显 bind password。当前版本提供配置校验和脱敏接口，尚未实现真实 LDAP bind/search/password authentication，因此该页面不能代表 LDAP 登录已经可用。

## 代码结构

```text
web/
├── index.html          Vite HTML 入口
├── src/main.js         状态、视图、API 请求、locale 和事件处理
├── src/style.css       页面布局和组件样式
├── package.json        npm scripts 和依赖
├── package-lock.json   锁定依赖版本
└── dist/               npm run build 生成的静态资源
```

前端保持单页应用结构，状态由 `src/main.js` 管理，API 错误通过页面 notice 展示。修改 API 字段时，应同步更新对应 view、loader、事件处理和 route integration test。

## 验证

```bash
cd web
npm ci
npm run build
```

仓库根目录的 Rust API 测试会通过真实 SQLite 和 Axum router 验证登录、cookie、设备、地址簿、用户组等接口。完整验证命令：

```bash
cargo test --locked --all-targets
cargo check --locked --all-targets
```

## 部署建议

- 优先由 API 和 Web 使用同一个 HTTPS origin，减少 cookie 和 CORS 配置复杂度。
- API 端口 `21114` 不要直接开放到公网。
- 关闭公开注册后再投入生产。
- 使用固定版本的 RustDesk server 镜像，不要把未经验证的 `latest` 当作生产版本。
- 不要把 JWT secret、OAuth client secret、LDAP bind password 或数据库文件提交到仓库。

跨源部署须在 API 设置 `API_ALLOWED_ORIGINS=https://web.example.com`，只接受精确 Origin（含端口），不支持通配。Web 构建设置 `VITE_API_BASE=https://api.example.com`。默认适合同站跨源；跨站须显式设置 `API_COOKIE_CROSS_SITE=1` 并使用 HTTPS。浏览器仍可能限制第三方 Cookie，此时页面会提示 Cookie 会话不可用，建议通过同源反向代理部署。Cookie 修改请求必须同时通过 Origin 和 `X-CSRF-Token` 校验；原生 Bearer 客户端不受浏览器 CSRF 协议影响。

浏览器回归：构建 API 和 Web 后运行 `RUSTDESK_API_BINARY=/absolute/path/rustdesk-api npm run test:browser`。测试使用独立临时数据库、随机端口和系统 Chrome，涵盖允许/拒绝跨源、登录恢复、修改、注销及第三方 Cookie 限制。

管理员设备页面分别展示未认证报告和已核验归属。先通过已有可信渠道从受控设备核对公钥，再选择账户并填写 SHA-256 指纹进行绑定；报告中的用户名或 UUID 不能作为归属证明。支持按 RustDesk ID 查询登记记录和解除绑定。历史关联保留为待核验，普通用户只看到公钥及 UUID 仍与登记记录匹配的已核验设备。
