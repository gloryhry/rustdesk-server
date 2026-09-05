# RustDesk Server

RustDesk Server 是 RustDesk 的自托管服务端项目，提供设备发现、NAT 穿透协调、Relay 中继，以及可选的兼容 HTTP API 和 Web 管理客户端。项目可以只运行传统的 `hbbs`/`hbbr` 服务，也可以额外启用 `rustdesk-api` 和 Web Admin，形成一套带用户、设备、地址簿和权限管理的独立部署。

[![build](https://github.com/rustdesk/rustdesk-server/actions/workflows/build.yaml/badge.svg)](https://github.com/rustdesk/rustdesk-server/actions/workflows/build.yaml)

- [发布版本](https://github.com/rustdesk/rustdesk-server/releases)
- [RustDesk 自托管文档](https://rustdesk.com/docs/en/self-host/)
- [环境变量完整说明](docs/environment-variables.md)
- [Web Client 说明](web/README.md)
- [Kubernetes 示例](kubernetes/README.md)

> 本项目面向需要自行控制服务端、数据和网络入口的部署场景。生产环境必须使用真实随机密钥、HTTPS 反向代理、受限的管理端口和可靠的持久化备份。

## 功能概览

### RustDesk 核心服务

- `hbbs`：ID Server/Rendezvous Server，负责设备注册、在线发现和连接协商。
- `hbbr`：Relay Server，在客户端无法直接建立连接时转发会话流量。
- 支持命令行参数、环境变量以及项目既有配置文件读取规则。
- 支持绑定地址、标准 RustDesk 端口、Relay 地址、日志级别、强制中继等配置。

### 独立 API 服务

`rustdesk-api` 是可选的独立 HTTP API 进程，默认关闭。它使用 SQLite 保存 API 用户、登录 session、设备、用户组、设备组、个人地址簿和标签数据，并提供与 `lejianwen/rustdesk-api` 常用接口兼容的路由。

主要能力包括：

- 用户注册、登录、当前用户查询、注销和持久化 session 撤销。
- JWT Bearer token 和 `rustdesk_api_token` HttpOnly cookie 两种认证方式。
- bcrypt 密码哈希、密码长度限制、用户名规范化和管理员权限检查。
- 设备上报、设备列表、设备状态展示和管理员设备删除。
- 管理员用户创建、状态修改、密码修改和删除。
- 管理员 session 列表与撤销。
- 用户组及成员管理。
- 设备组及设备成员管理。
- 个人地址簿的兼容 JSON snapshot、结构化 peer 条目和标签管理。
- OAuth/OIDC 登录，包含 state、PKCE、nonce，以及可选的 issuer、audience、expiry 和 JWKS RSA 签名校验。
- 管理员 LDAP 配置查看和校验接口。
- `/health/live` 健康检查、server config 和 stock RustDesk client 常用兼容路由。

API 监听地址和端口默认由 `API_BIND`、`API_PORT` 控制，示例端口是 `21114`。完整路由和环境变量请参考 [`docs/environment-variables.md`](docs/environment-variables.md)。

### Web Admin / Web Client

`web/` 是基于 Vite 的前端应用，既可以单独开发，也可以构建后由 `rustdesk-api` 的静态文件服务提供。当前页面包括：

- 登录、注册和配置的 OAuth/OIDC provider 登录入口。
- HttpOnly cookie session 恢复、注销和当前用户资料。
- 服务器公开配置展示。
- 地址簿结构化 peer 添加、编辑、删除，以及 raw JSON 高级编辑器。
- 地址簿标签创建、颜色设置和删除。
- 用户、用户组和用户组成员管理。
- 设备、设备组和设备组成员管理。
- 管理员设备删除、session 撤销、OAuth provider 状态和 LDAP 配置。
- `zh-CN`/`en-US` 切换。

## 项目结构

```text
.
├── src/
│   ├── api.rs              独立 HTTP API、认证、兼容路由和静态文件服务
│   ├── api_main.rs         rustdesk-api 二进制入口
│   ├── api_config.rs       API/OAuth 配置读取
│   ├── auth.rs             用户、密码、JWT 和 session 管理
│   ├── database.rs         SQLite 数据访问和迁移
│   ├── oauth.rs            OAuth/OIDC state、PKCE、nonce 和 JWKS 校验
│   ├── ldap.rs             LDAP 配置模型和安全校验
│   ├── main.rs              hbbs 入口
│   └── hbbr.rs              hbbr 入口
├── libs/hbb_common/        RustDesk 公共协议和基础库
├── web/                    Vite Web Admin/Web Client
├── tests/                  API route 和协议兼容集成测试
├── docs/                   环境变量和部署说明
├── systemd/                hbbs、hbbr、rustdesk-api 服务单元
├── debian/                 Debian 包模板和安装脚本
├── docker/                 s6 多服务 Docker 镜像构建上下文
├── docker-classic/         传统 scratch Docker 镜像构建上下文
├── kubernetes/             Kubernetes 单实例示例
├── .github/workflows/      测试、构建、Release 和 GHCR 自动化
├── Cargo.toml              Rust workspace 根配置
└── docker-compose.yml      本地 Docker Compose 示例
```

## 工作原理和数据

传统部署中，客户端连接 `hbbs` 完成设备发现和连接协调；需要中继时，流量转由 `hbbr` 转发。API 是独立进程，不参与 RustDesk 核心连接协议，可以和 `hbbs`/`hbbr` 使用同一数据目录，也可以单独使用自己的数据库路径。

API 登录成功后同时返回 JWT access token，并设置 `rustdesk_api_token` cookie。浏览器使用 HttpOnly cookie 可以在刷新页面后恢复 session；原生客户端或脚本可以使用 JSON 返回的 Bearer token。服务端仍会检查数据库 session 是否有效，因此注销或管理员撤销 session 后，原 token 不能继续使用。

API 数据默认使用 SQLite。生产环境必须保证数据库目录可写、具备持久化卷，并在升级前备份。Kubernetes 示例采用单副本 `Recreate` 策略和 `ReadWriteOnce` PVC，不是高可用配置。

## 环境要求

开发和本地构建至少需要：

- Rust stable toolchain 和 Cargo。
- Node.js 18 或更新版本，以及 npm。
- Linux 构建通常需要 `pkg-config`、C 编译器和 Rust 依赖对应的系统库。
- Docker 构建需要 Docker daemon 和 BuildKit/buildx。
- Debian 包构建需要 `dpkg-dev debhelper devscripts fakeroot build-essential pkg-config`。
- Kubernetes 部署需要可用的 Kubernetes 集群；验证 manifest 时建议安装 `kubectl` 或 `kubeconform`。

## 快速开始

### 1. 构建服务端

```bash
cargo build --release --bins
```

产物位于 `target/release/`：

- `hbbs`
- `hbbr`
- `rustdesk-api`
- `rustdesk-utils`

### 2. 启动核心服务

开发环境可以直接运行：

```bash
./target/release/hbbs -r 127.0.0.1:21117
./target/release/hbbr
```

生产环境建议使用 [`systemd/`](systemd/) 中的服务单元，并把生成的 key、日志和数据库放在受限且持久化的目录中。客户端需要配置公开可访问的 ID Server 和 Relay Server 地址。

### 3. 启用 API

API 默认关闭。至少需要设置 32 字节的 JWT secret，并指定数据库和 key 文件：

```bash
export API_ENABLED=1
export API_BIND=127.0.0.1
export API_PORT=21114
export API_JWT_SECRET='replace-with-at-least-32-random-bytes'
export DB_URL="$PWD/data/db_v2.sqlite3"
export RUSTDESK_KEY_FILE="$PWD/data/id_ed25519.pub"
export API_REGISTER_ENABLED=1
mkdir -p data
./target/release/rustdesk-api
```

第一次部署建议通过 `API_BOOTSTRAP_ADMIN_USERNAME` 和 `API_BOOTSTRAP_ADMIN_PASSWORD` 创建管理员。生产环境完成初始化后应关闭公开注册：

```bash
export API_BOOTSTRAP_ADMIN_USERNAME=admin
export API_BOOTSTRAP_ADMIN_PASSWORD='use-a-long-random-password'
export API_REGISTER_ENABLED=0
```

API 健康检查：

```bash
curl http://127.0.0.1:21114/health/live
```

### 4. 构建和运行 Web Client

开发模式：

```bash
cd web
npm ci
npm run dev
```

打开 `http://127.0.0.1:5178`。Vite 默认把 `/api` 代理到 `http://127.0.0.1:21114`。如果 API 位于其他地址：

```bash
VITE_API_PROXY_TARGET=http://127.0.0.1:21114 npm run dev
```

生产构建：

```bash
cd web
npm ci
npm run build
```

静态产物写入 `web/dist`。可以由 HTTPS 反向代理提供，并把 `/api` 转发到 API；也可以让 API 进程直接提供静态文件：

```bash
API_WEB_ROOT=/absolute/path/to/rustdesk-server/web/dist \
./target/release/rustdesk-api
```

如果前端和 API 不同源，构建时设置：

```bash
VITE_API_BASE=https://api.example.com npm run build
```

生产浏览器访问必须使用 HTTPS。完整 Web 说明见 [`web/README.md`](web/README.md)。

## API 使用示例

注册普通用户：

```bash
curl -i -X POST http://127.0.0.1:21114/api/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"a-long-password"}'
```

登录并保存 cookie：

```bash
curl -i -c cookies.txt -X POST http://127.0.0.1:21114/api/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"a-long-password"}'
```

使用 cookie 查询当前用户：

```bash
curl -b cookies.txt http://127.0.0.1:21114/api/currentUser
```

使用 Bearer token：

```bash
curl -H "Authorization: Bearer $ACCESS_TOKEN" \
  http://127.0.0.1:21114/api/peers
```

注销并撤销当前 session：

```bash
curl -b cookies.txt -X POST http://127.0.0.1:21114/api/logout
```

管理员接口必须使用管理员 session。不要把 JWT secret、管理员密码、OAuth client secret 或 LDAP bind password 写入仓库、镜像层或公开 issue。

## 配置

完整环境变量表、优先级、数据库和 OAuth/OIDC 说明见 [`docs/environment-variables.md`](docs/environment-variables.md)。常用核心配置如下：

| 配置 | 用途 |
| --- | --- |
| `API_ENABLED` | 是否启用独立 API，默认关闭 |
| `API_BIND` / `API_PORT` | API 监听地址和端口 |
| `API_JWT_SECRET` | JWT 签名密钥，至少 32 字节 |
| `API_BOOTSTRAP_ADMIN_USERNAME` / `API_BOOTSTRAP_ADMIN_PASSWORD` | 首次创建管理员 |
| `API_REGISTER_ENABLED` | 是否允许公开注册普通用户 |
| `DB_URL` | SQLite 数据库路径 |
| `API_WEB_ROOT` | API 直接提供的 Web 静态文件目录 |
| `RUSTDESK_ID_SERVER` / `RUSTDESK_RELAY_SERVER` | 对外公布的核心服务地址 |
| `RUSTDESK_KEY_FILE` | RustDesk 公钥文件路径 |
| `API_OAUTH_REDIRECT_URL` | OAuth/OIDC 固定 callback URL |
| `API_OIDC_ISSUER_URL` / `API_OIDC_JWKS_URL` | OIDC ID token 验证 endpoint，必须成对设置 |
| `API_LDAP_*` | LDAP 配置；当前提供配置校验接口 |

### OAuth/OIDC

OAuth callback 使用服务端保存的一次性 state 和 PKCE code verifier，避免接受客户端提交的任意 redirect URL。浏览器 callback 成功后会设置 HttpOnly、SameSite=Lax cookie 并跳转到 Web 首页；API/native caller 可以读取 JSON token。

通用 OIDC provider 如果配置 `API_OIDC_ISSUER_URL` 和 `API_OIDC_JWKS_URL`，token endpoint 返回的 `id_token` 会经过签名、issuer、audience、expiry、subject 和 nonce 验证。远程 endpoint 必须使用 HTTPS；只有本机回环地址允许 HTTP。

### LDAP

当前版本已实现 LDAP 配置模型、字段校验、密码脱敏和管理员配置接口，但尚未实现真实 LDAP bind/search/password authentication。启用 LDAP 前不要把配置层能力误认为已经提供完整 LDAP 登录。

## Docker

`docker/Dockerfile` 构建基于 s6-overlay 的多服务镜像，镜像内包含 `hbbs`、`hbbr`、`rustdesk-api` 和 `web/dist`。API 仍默认关闭，使用 `API_ENABLED=1` 显式启用。镜像暴露以下常用端口：

- `21114/tcp`：API。
- `21115/tcp`、`21116/tcp+udp`、`21118/tcp`：hbbs。
- `21117/tcp`、`21119/tcp`：hbbr。

本地 Compose 示例：

```bash
export API_JWT_SECRET='replace-with-at-least-32-random-bytes'
docker compose up -d hbbr hbbs
# 需要 API 时启用 api profile
docker compose --profile api up -d rustdesk-api
```

Compose 将 `./data` 挂载到容器数据目录。生产环境应使用 GHCR 发布的固定版本 tag，而不是未经验证的 `latest`，并通过 HTTPS 反向代理保护 API 和 Web。

自动发布的镜像地址和 tag 规则见下方“CI/CD 和发布”。

## Debian 安装和 systemd

仓库包含 `rustdesk-server-hbbs`、`rustdesk-server-hbbr`、`rustdesk-server-utils` 和 `rustdesk-server-api` 包模板。构建 Debian 包需要：

```bash
sudo apt install dpkg-dev debhelper devscripts fakeroot build-essential pkg-config
```

CI 使用 release 二进制 artifact 创建 Linux amd64 `.deb`。本地可参考 [`debian/README.source`](debian/README.source)，并在生成 `debian/control` 后运行：

```bash
debuild -uc -us -b
```

安装后使用 [`systemd/`](systemd/) 中的服务单元管理进程。服务运行用户、数据目录、数据库路径和 key 文件应按发行版安全策略调整；不要把数据库和 key 放在临时目录。

## Kubernetes

示例 [`kubernetes/example.yaml`](kubernetes/example.yaml) 将 `hbbs`、`hbbr` 和 `rustdesk-api` 放在一个 `Recreate` Deployment 中，共享一个 `ReadWriteOnce` PVC，并暴露核心 RustDesk 端口和 API `21114`。

应用前必须：

1. 替换 Secret 中的 `API_JWT_SECRET`，使用至少 32 字节随机值。
2. 把 hbbs 命令中的 Relay 主机名改为实际公开地址。
3. 配置 `RUSTDESK_ID_SERVER`、`RUSTDESK_RELAY_SERVER` 和 `API_PUBLIC_URL` 等实际公开地址。
4. 提供 `web/dist`，或者将 Web 资源挂载到 `/root/web`。
5. 在 HTTPS 反向代理后暴露管理端和 Web。

```bash
kubectl apply -f kubernetes/example.yaml
kubectl get pods -l app=rustdesk
kubectl get service rustdesk-service
```

该示例是单实例参考部署，不适合直接扩展为 HA。SQLite、PVC 锁和 `Recreate` 策略必须在扩容前替换为适合多副本的数据库和存储设计。

## 端口、备份和安全

- API `21114` 不应直接暴露到互联网，应放在 HTTPS reverse proxy 后面并限制来源。
- RustDesk 核心端口应只开放实际需要的 TCP/UDP 入口。
- `API_JWT_SECRET`、管理员密码、OAuth secret、LDAP bind password 和 RustDesk key 必须使用 secret manager 或受限文件保存。
- 备份 SQLite 数据库前暂停写入或采用一致性备份方式，同时保存 RustDesk key。
- 升级前阅读 release notes，先在备份或测试环境执行 migration 和 smoke test。
- 关闭公开注册，限制管理员账号，定期撤销旧 session。
- 浏览器部署使用 HTTPS；不要在日志中输出 access token、cookie 或密码。

## 测试和 CI

本地运行完整 Rust 验证：

```bash
cargo test --locked --all-targets
cargo check --locked --all-targets
cargo build --locked --release --bins
```

验证 Web：

```bash
cd web
npm ci
npm run build
```

API compatibility workflow 会运行 Rust tests、API binary build、API process smoke test 和 Web build。发布 workflow 在主分支推送时创建 Pre-release，在版本 tag 推送时创建正式 Release，并把 Linux amd64 Docker 镜像发布到 GHCR；仓库中的旧跨架构构建 workflow 仍可手动运行。

## 自动发布

GitHub Actions 发布工作流约定如下：

- `master` 推送：构建并创建 `pre-<commit-sha>` Pre-release，同时发布 GHCR `edge` 镜像和不可变 commit tag。
- 版本 tag 推送：构建并创建正式 Release，同时发布版本和 `latest` GHCR tag。
- Pull Request：只验证，不创建 Release、不推送镜像。
- `workflow_dispatch`：默认只验证/构建，避免手动运行意外发布。

GHCR 镜像以当前仓库 owner 命名，通常是：

```text
ghcr.io/<github-owner>/rustdesk-server
ghcr.io/<github-owner>/rustdesk-server-classic
```

实际 tag 和构建平台以 `.github/workflows/` 中的 workflow 为准。

## 许可证和上游资料

本项目遵循仓库中的许可证文件。RustDesk 官方自托管文档、FAQ 和 Pro 产品信息请参考：

- <https://rustdesk.com/docs/en/self-host/>
- <https://github.com/rustdesk/rustdesk/wiki/FAQ>
- <https://github.com/rustdesk/rustdesk-server-demo>
- <https://rustdesk.com/pricing.html>
