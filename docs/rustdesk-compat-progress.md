# RustDesk 兼容性与安全修复台账

基线：`0f30105`。分支：`codex/rustdesk-client-compat-fixes`。
目标：官方 RustDesk 1.4.9 相关 HTTP 协议与旧版个人地址簿。
每项修复单独中文提交；不推送、不迁移生产数据库、不部署生产服务。

## 执行状态

| 顺序 | 原问题 | 修复内容 | 状态 |
|---|---|---|---|
| 01 | #6 | 认证 Cookie 安全属性与本机开发例外 | 已完成 |
| 02 | #1 | OAuth 浏览器绑定与回调重放 | 已完成 |
| 03 | #4 | Google 与通用 OIDC 配置 | 已完成 |
| 04 | #5 | 强制 OIDC 身份令牌验证 | 已完成 |
| 05 | #3 | 原生 OAuth 发起与轮询 | 待处理 |
| 06 | #15 | OAuth provider 管理与持久化 | 待处理 |
| 07 | #18 | CORS、Cookie 会话和 CSRF | 待处理 |
| 08 | #7 | 设备报告、绑定和注册时间 | 待处理 |
| 09 | #9 | 官方设备 DTO | 待处理 |
| 10 | #8 | 列表分页与 total | 待处理 |
| 11 | #16 | 用户列表账户 ID | 待处理 |
| 12 | #12 | 地址簿局部编辑保留字段 | 待处理 |
| 13 | #10 | 删除快照条目 | 待处理 |
| 14 | #17 | 修改 Peer ID | 待处理 |
| 15 | #13 | forceAlwaysRelay 编码 | 待处理 |
| 16 | #14 | tag_colors 编码与保留 | 待处理 |
| 17 | #11 | 统一地址簿存储与迁移 | 待处理 |
| 18 | #2 | 新版个人地址簿完整协议 | 待处理 |
| 19 | #19 | 自构建镜像与部署验收 | 待处理 |

## 验收记录

所有 Rust 命令使用 `CARGO_TARGET_DIR=/tmp/rustdesk-review-target`。当前缓存可用，使用 `--offline`；依赖有变时另外验证在线解析。
网络测试在允许临时回环端口的环境执行；沙箱内绑定端口被拒绝不是通过证据。

### 01 / 原 #6

- 复现：在 HTTP 登录集成测试新增 Secure 属性断言，修复前因缺少 Secure 失败。
- 修改：所有认证 Cookie 使用统一策略；默认 Secure、HttpOnly、SameSite=Lax、Path=/，无 Domain；注销采用相同策略。
- 开发配置：`API_ALLOW_INSECURE_LOCAL_HTTP=1` 仅允许回环监听与本机 HTTP 公共 URL；转发请求头不能降级策略。
- 测试：普通及管理员登录/注销、OAuth 回调、伪造转发头、本机 HTTP 例外、非本机配置拒绝。
- 数据隔离：原有 10000 条 Peer 写入测试改用独立临时目录，并检查所有任务执行结果。
- 验收：`cargo test --locked --offline --all-targets` 全部 33 项通过；`cargo check --locked --offline --all-targets` 通过。数据库和锁文件 SHA-256 与基线一致。
- 基线 `test.sqlite3` SHA-256：`2b4fba82e84e89de33d16a71df3739e24e67204b12bf62b0de12148d211b9a6e`。
- 基线 `Cargo.lock` SHA-256：`4bdb70ae0c422e1b63850b350f23807d1d4be5090d4df267082b844fd5618821`。

### 02 / 原 #1

- 复现：无发起浏览器 Cookie 的 HTML 回调在修复前返回 307 并建立登录，回归要求返回 400。
- 修改：每个 state 使用独立随机浏览器 Cookie，服务端仅保存 SHA-256 摘要；验证绑定和固定回调 URI 后原子领取 state。
- 生命周期：300 秒有效期、最多 10000 个待处理 state；成功回调删除绑定 Cookie；多标签互不覆盖。保留 PKCE 与 nonce。
- 测试：合法绑定、无 Cookie、错误浏览器、重放、并发领取、精确过期边界、错误 redirect URI 不消耗授权；拒绝结果无认证 Cookie。
- 时间边界通过注入时钟验证，不使用等待 300 秒或访问生产服务。
- 验收：根包 38 项测试与全部目标检查通过；数据库及锁文件校验值与基线一致。

### 03 / 原 #4

- 复现：配置 openid scope 却同时缺少 issuer/JWKS 的提供方在修复前被接受，回归要求拒绝。
- 修改：显式 OAuth2/OIDC 类型；GitHub 为 OAuth2，Google 为 OIDC 并配置官方 issuer/JWKS；通用 OIDC 缺少任一必需项时启动报告错误。
- 配置验证使用独立读取接口测试，避免测试修改进程环境影响并行测试；错误消息不含 client secret。
- 新增测试：Google 固定验证端点、通用 OIDC 缺项逐项补齐、部分凭据错误、空配置合法、GitHub 类型；本地签名/JWKS 模拟提供方验证 Google issuer 的有效令牌能成功登录。
- 验收：根包 43 项测试及全部目标检查通过；数据库与锁文件校验值保持一致。测试 RSA 密钥仅供本地模拟提供方使用，公开固定测试样本不可用于真实服务。

### 04 / 原 #5

- 复现：完整 OIDC 配置收到无 ID token 的 token 响应，修复前仍返回身份；新增回归断言先失败。
- 修改：OIDC 必须验证 ID token；OAuth2 独立使用 userinfo。验证签名、算法、kid、issuer、audience、exp、nbf、nonce 与 sub；多 audience 必须有匹配 azp。
- 主体一致性：OIDC userinfo.sub 必须为字符串并精确等于已验证 ID token.sub，不接受 id 回退。
- 测试：缺失令牌、篡改签名、错误算法/kid、错误及缺失声明、过期、nbf、多个 audience、不同 userinfo 主体；合法签名和授权方正例。HTTP 拒绝后账户与会话列表均未新增。
- 验收：根包 49 项测试及全部目标检查通过，现有数据库和锁文件校验值保持一致。

## 完成与发布边界

Goal 完成需 19 项独立提交和所有必选自动化检查通过。提交 SHA 通过 `git log --grep='问题：原 #'` 查询，避免在提交自身写入自身 SHA。
最终仍需官方 Windows/Linux 1.4.9 客户端实际登录、地址簿、直连、中继、传输和重连验收。
实际客户端联调是发布门禁；没有执行时不得宣称所有功能完美兼容。
