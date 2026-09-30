# 源码镜像部署与验收

官方 rustdesk-server 镜像不包含本仓库 API，不能替代本镜像。示例固定 `rustdesk-local/server:1.1.17-api-1.4.9`，生产应推送到自己的 registry 并指定验证过的版本或 digest。构建上下文必须为仓库根目录，包含相匹配的 hbb_common 子模块：

```bash
git submodule update --init --recursive
docker build -f docker/Dockerfile --target runtime -t rustdesk-local/server:1.1.17-api-1.4.9 .
docker build -f docker/Dockerfile --target supervisor -t rustdesk-local/server:1.1.17-api-1.4.9-s6 .
```

runtime 使用非 root UID/GID 10001，包含 CA、OpenSSL、C++ 运行库；s6 入口需要 root 管理服务。runtime 默认 API_ENABLED=0，Compose/Kubernetes 显式启用 API；s6 默认启用。`docker-classic/Dockerfile` 保留等价单进程源码构建入口，CI 统一使用主 Dockerfile 的 runtime target。

## 初始化配置

在权限 0600 的配置文件或 secret manager 中保存秘密，不放入镜像层或命令输出：

- `API_JWT_SECRET`：独立随机签名秘密，至少 32 字节。
- `API_OAUTH_CONFIG_KEY`：独立随机 32 字节的 base64 值，用于提供方秘密的认证加密。必须备份，不能丢失或在重启时随机替换。
- `API_BOOTSTRAP_ADMIN_USERNAME/PASSWORD`：新数据库必需，已有管理员后可移除。
- `API_PUBLIC_URL`：实际 HTTPS API 地址；`RUSTDESK_RELAY_SERVER` 为实际中继地址；按需要设置 `RUSTDESK_ID_SERVER`、`API_OAUTH_REDIRECT_URL` 和精确 `API_ALLOWED_ORIGINS`。

`rustdesk-api --initialize` 在不监听端口的情况下验证密钥对、Web 资源、全部 API 配置并执行版本化迁移和管理员初始化。容器入口通过 flock 串行执行；Kubernetes 使用 initContainer。现有私钥缺少对应公钥或密钥不匹配会报错，不能通过删钥重试，以免意外轮换服务身份。

数据库、RustDesk 私钥/公钥统一放在 `/data`；Web 成品位于 `/usr/share/rustdesk-api-web`。用户数据库必须用一致性备份预检迁移，不能将生产卷交给测试脚本。API readiness `/health/ready` 验证数据库可访问、迁移版本和 Web 成品，失败为 503；`/health/live` 独立用于存活检查。

## 自动化验收

```bash
python3 tests/deployment/run.py
# 已构建镜像时可用 --skip-build；支持 --kind 和 --kubectl 指定工具路径
```

脚本生成随机秘密、回环端口、临时 Compose project/卷和 kind 集群；测试三个服务、静态页、登录、固定官方 1.4.9 地址簿序列、真实 UDP 公钥注册、无认证报告、人工指纹绑定、OAuth mock 和重启持久化。还验证缺失秘密无法初始化、资源缺失使 readiness 失败，以及 Kubernetes server-side 清单检查、Pod 重建和 PVC 持久化。s6 使用独立卷运行相同检查。异常结束时执行资源清理，不 prune 其他容器/卷。

自动化协议测试不代表 Windows/Linux 客户端实机联调；实际直连、中继、文件传输等仍需发布门禁。

## 升级与回滚

1. 暂停所有写入，保存一致的 SQLite 备份、RustDesk 密钥、OAuth 配置加密密钥及部署配置。
2. 使用备份副本运行新版本初始化与协议验收，检查迁移冲突报告；有冲突先人工修复副本，禁止默默丢弃记录。
3. 安排维护窗口后运行生产初始化，再检查 readiness 与正式客户端门禁。
4. 回滚恢复**匹配的程序版本和数据库备份**，并保持对应密钥。不能只降级镜像后继续读取已升级数据库。

本轮开发不执行生产迁移或发布。
