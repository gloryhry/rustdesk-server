# RustDesk 兼容性与安全修复交付台账

基线 `0f30105`，分支 `codex/rustdesk-client-compat-fixes`。最终修复源码提交 `4de82bd`。
原计划 19 项与执行中发现的 19 项均已独立中文提交，共 38 个修复提交；此文档汇总不属于新增修复问题。
没有 push、生产迁移或上线。

**官方 1.4.9 相关协议与旧版个人地址簿自动化验收通过。**
这是明确版本和接口范围的结论；Windows/Linux 未修改官方客户端实机联调尚未执行，仍是发布门禁。
不包含 LDAP 登录、完整 Pro 共享地址簿、所有历史客户端或所有 Rust 编译平台。

## 问题、提交与验收

测试名对应仓库 `tests/*.rs`；内部模块单测另行标明。每项先证明失败，再修复并执行专项、根包测试与 locked check；涉及 Web 的主修复还执行 Web 构建/浏览器回归。详细历史结果见 [执行台账](rustdesk-compat-progress.md) 和 [依赖审计记录](dependency-audit.md)。最终全套检查再次覆盖全部提交。

| 执行顺序 | 问题编号 | 中文提交标题 | SHA | 回归测试/审计 | 结果 |
| --- | --- | --- | --- | --- | --- |
| 01 | 原 #6 | 修复：统一认证 Cookie 安全属性与开发例外 | `3ea16d48c262` | browser_security、api_routes；普通/管理员/OAuth Cookie 与隔离数据库 | 通过 |
| 02 | 原 #1 | 修复：绑定 OAuth 登录浏览器并阻止回调重放 | `56449eeb9307` | oauth_browser；跨浏览器、过期、重放与并发回调 | 通过 |
| 03 | 原 #4 | 修复：补全 Google OIDC 配置并校验提供方类型 | `d1d23d138b2e` | oauth::tests、oidc_validation；Google 签名正例与缺项配置 | 通过 |
| 04 | 原 #5 | 修复：强制执行 OIDC 身份令牌与主体一致性验证 | `0e2c5fffc6a1` | oidc_validation；签名/算法/issuer/audience/nonce/subject 攻击与正例 | 通过 |
| 05 | 原 #3 | 修复：实现官方客户端 OAuth 发起与轮询登录协议 | `7e105c1d5233` | native_oauth；官方发起→浏览器→等待/领取/并发时序 | 通过 |
| 06 | 原 #15 | 修复：补齐 OAuth 提供方管理与安全持久化 | `e53b33788821` | oauth_admin；权限、秘密加密、重启、停用；Chrome provider CRUD | 通过 |
| 07 | 原 #18 | 修复：支持受控跨源会话并校验浏览器修改请求 | `0da0a4a94975` | browser_sessions；Origin/CSRF/Bearer；Chrome 跨源与刷新 | 通过 |
| 08 | 原 #7 | 修复：接收官方设备报告并隔离设备归属验证 | `cd1d304d0b6d` | device_reports；真实 UDP、无认证报告、指纹绑定、队列与在线过期 | 通过 |
| 09 | 原 #9 | 修复：按官方协议返回设备标识与信息对象 | `a92d48ffe206` | device_dto；官方结构、内部 ID、账户与组关系隔离 | 通过 |
| 10 | 原 #8 | 修复：补齐官方列表分页总数与筛选语义 | `0d01091ef49e` | list_pagination；0/1/100/101、筛选、稳定排序；Chrome 跨页选择 | 通过 |
| 11 | 原 #16 | 修复：补全用户标识并恢复用户组成员选择 | `0b2986c202a5` | user_list；账户 ID 与组权限；Chrome 本人入组 | 通过 |
| 12 | 原 #12 | 修复：地址簿局部编辑保留密码与扩展字段 | `6a3b58a439a8` | address_book_edits；局部修改保留连接/扩展字段与回滚；Chrome | 通过 |
| 13 | 原 #10 | 修复：允许删除仅存在于快照的地址簿条目 | `6a3a4579bc51` | address_book_edits；快照条目删除、跨账户、失败回滚；Chrome | 通过 |
| 14 | 原 #17 | 修复：按稳定条目标识更新地址簿设备编号 | `e9339422fbf6` | address_book_edits；稳定 entry ID、Peer ID 改号与 409；Chrome | 通过 |
| 15 | 原 #13 | 修复：兼容官方强制中继字段的字符串格式 | `94b519a85567` | address_book_edits、codec；布尔/字符串往返与非法值拒绝；Chrome | 通过 |
| 16 | 原 #14 | 修复：统一地址簿标签颜色编码并防止数据丢失 | `1706c51a591b` | address_book_edits、codec；ARGB/alpha/标签引用与损坏数据；Chrome | 通过 |
| 17 | 原 #11 | 修复：统一地址簿存储并消除快照与索引分裂 | `1fbecd77d6d0` | address_book_store；历史并集迁移、CAS、索引重建、事务回滚；Chrome | 通过 |
| 18 | 原 #2 | 修复：完整适配官方新版个人地址簿协议 | `34243e6e2ddb` | official_address_book；固定 1.4.9 样本、空 body、CRUD、GUID 权限 | 通过 |
| 19 | 原 #19 | 修复：使用自构建镜像并验证完整服务部署 | `c003260d20c7` | deployment_initialization/readiness；源码镜像、Compose/s6/kind smoke | 通过 |
| 20 | 新 #20 | 修复：公钥持久化失败时返回真实注册错误 | `20d032eafed5` | device_reports::real_hbbs_failed_key_persistence_returns_error_and_retry_does_not_use_failed_cache | 通过 |
| 21 | 新 #21 | 修复：升级 Axum 并限制默认 JSON 请求体 | `2a6445aa5a92` | security_dependencies；JSON 超限返回 413；RUSTSEC-2022-0055 | 通过 |
| 22 | 新 #22 | 修复：升级 Crossbeam 消除非法指针解引用漏洞 | `5cf07008d019` | 定向 RUSTSEC-2026-0204 审计 | 通过 |
| 23 | 新 #23 | 修复：升级 OpenSSL 绑定消除释放后使用漏洞 | `e858d4ef5d50` | 定向 RUSTSEC-2025-0022、RUSTSEC-2025-0004 审计 | 通过 |
| 24 | 新 #24 | 修复：移除临时目录清理的符号链接竞争漏洞 | `6fdf44d5d2f1` | 定向 RUSTSEC-2023-0018 审计与协议重新生成 | 通过 |
| 25 | 新 #25 | 修复：升级 rustls 拒绝错误加密边界的握手消息 | `eb6f41a1bb4d` | 定向 RUSTSEC-2026-0285 审计 | 通过 |
| 26 | 新 #26 | 修复：升级 WebSocket 库消除拒绝服务漏洞 | `23a14f240606` | websocket_protocol；真实 protobuf 回复；RUSTSEC-2023-0065 | 通过 |
| 27 | 新 #33 | 修复：隔离 hbbs 回归测试的客户端配置目录 | `9185f9db7c9c` | test_environment；原生与禁止外连的内部网络容器红绿配置隔离 | 通过 |
| 28 | 新 #28 | 修复：升级 SQLite 数据库栈并移除旧 TLS 漏洞依赖 | `d8312fe13517` | 数据库/迁移/回滚；RUSTSEC-2022-0090、2024-0363、2024-0336、2023-0052；RSA 无新增公告 | 通过 |
| 29 | 新 #29 | 修复：升级 HTTP 栈并移除旧协议与证书验证漏洞 | `5627fd9cb625` | http_transport；真实 HTTP/1.1/2 与静态页/413；HTTP 四条定向审计；Chrome 15 项 | 通过 |
| 30 | 新 #27 | 修复：升级 JWT 签名依赖并移除旧 ring 漏洞版本 | `319210520591` | oidc_validation；定向 RUSTSEC-2025-0009 审计 | 通过 |
| 31 | 新 #30 | 修复：升级 Wayland 生成器消除 XML 解析漏洞 | `ea10f166342b` | Wayland 代码生成；RUSTSEC-2026-0194、RUSTSEC-2026-0195 | 通过 |
| 32 | 新 #31 | 修复：替换旧 users 实现并消除虚假 root 组成员 | `47aac5730a0f` | unix_groups；真实 id -G 对照与 UID；RUSTSEC-2025-0040 | 通过 |
| 33 | 新 #34 | 修复：升级 anyhow 并将内存安全公告纳入审计门禁 | `9af886af579a` | 审计脚本捕获 unsound；RUSTSEC-2026-0190；CI --deny unsound | 通过 |
| 34 | 新 #36 | 修复：升级 bumpalo 消除迭代器生命周期漏洞 | `639815472b2a` | 定向 RUSTSEC-2022-0078 审计 | 通过 |
| 35 | 新 #37 | 修复：移除未使用配置解析中的内存安全漏洞依赖 | `28694a3a9e8f` | 连接池/并发/回滚；定向 RUSTSEC-2023-0086 审计 | 通过 |
| 36 | 新 #38 | 修复：升级 rand 消除日志重入的引用别名漏洞 | `68280144c37e` | 随机/协议回归；定向 RUSTSEC-2026-0097 审计 | 通过 |
| 37 | 新 #35 | 修复：使用标准库终端检测替代不安全的 atty 实现 | `91cb8d0aef0f` | terminal_cli；真实管道/伪终端；RUSTSEC-2021-0145 | 通过 |
| 38 | 新 #32 | 修复：限制 WebSocket 转发地址仅来自显式可信代理 | `4de82bd7fdd4` | websocket_proxy；3 策略单测＋5 真实 hbbs/hbbr 测试；完整部署 | 通过 |

## 最终实际执行结果

- Rust：155 项根包测试通过，locked 全目标 check 通过，hbbs/hbbr/rustdesk-api 构建通过。`--all-targets` 指当前 Linux 工具链的包目标；没有把它称为 Windows/WASM 交叉编译。
- 协议：固定官方 1.4.9 来源 `6c578292e8ebbbec708b76986ba8c4bc7c509747` 的样本与 DTO 验证；初始化完整时序、Peer/标签 CRUD、分页、越权、新旧交替读写通过。测试不是仅检查 HTTP 200。
- OAuth：本地授权/token/userinfo/JWKS mock、合法流程及 CSRF/签名/nonce/重放/并发/主体不一致等攻击回归通过。
- 数据：新库、历史/重复迁移、冲突/损坏数据、回滚、跨账户和并发 CAS 通过。所有测试使用临时数据、临时密钥与随机回环端口。
- Web：构建通过，真实 Chrome 15 项通过，覆盖会话恢复、跨源/CSRF、provider、组选择与地址簿。
- 部署：重新构建 release runtime 与 s6 镜像；临时 Compose/s6/kind v0.30.0、Kubernetes v1.34.0 全流程通过，包含 server-side 清单校验、所有服务端口、readiness/liveness、负例、真实注册/报告/绑定、OAuth mock、重启和 Pod/PVC 持久化。该轮专属集群、容器、网络、卷均清理且只读核验为空。
- 安全审计：公告库已刷新，无 ignore；Rust 漏洞 0、unsound 0，npm 生产依赖漏洞 0。仍有 ansi_term、bincode、dlopen_derive、sodiumoxide 四条停止维护提示，属于维护风险，未标为已修复漏洞。

实际命令（缓存目标目录不影响测试隔离）：

```bash
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo test --locked --offline --all-targets
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo check --locked --offline --all-targets
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo build --locked --offline --bin hbbs --bin hbbr --bin rustdesk-api
(cd web && npm run build)
(cd web && RUSTDESK_API_BINARY=/tmp/rustdesk-review-target/debug/rustdesk-api npm run test:browser)
/tmp/rustdesk-audit-tools/bin/cargo-audit audit --deny unsound --json
(cd web && npm audit --omit=dev --audit-level=low --json)
python3 tests/deployment/run.py --kind /tmp/rustdesk-deployment-tools/kind --kubectl /tmp/rustdesk-deployment-tools/kubectl
```

本次原始结果保留在执行环境 `/tmp/rustdesk-fix32-tests.log`、`/tmp/rustdesk-fix32-check.log`、`/tmp/rustdesk-final-services.log`、`/tmp/rustdesk-final-web.log`、`/tmp/rustdesk-final-browser.log`、`/tmp/rustdesk-final-deployment.log` 与最终 Rust/npm 审计 JSON；这些临时日志不作为重新验收的替代。

公告库：`9b3a3b73a7f42606494c943e95f8196e9994df46`，更新于 `2026-09-30T09:15:39+02:00`。审计结论以该库为准。

本地已验收镜像：

- `rustdesk-local/server:1.1.17-api-1.4.9`：`sha256:6fc310c29aac837e56f727bf94e78c1c7e076063ab7160c6d711562468e59d3c`
- `rustdesk-local/server:1.1.17-api-1.4.9-s6`：`sha256:68dc2232c83d22c39ac757b85b5bbf868a680b53897f7fb23d3413759f0f0b87`

以上为本地镜像 ID，未推送 registry，不能把它们当作已发布的 registry digest。

## 兼容接口与配置

- 官方 API：密码/AuthBody、原生 OAuth 发起与一次性轮询、sysinfo/heartbeat、peers/users/accessible groups 的 total/data、完整新版个人地址簿协议；旧版 `/api/ab` 整本读写保留。共享 profiles 明确返回空列表，不声称 Pro 共享能力。
- Web 条目与标签管理使用 `/api/web/ab/...`；携带 revision 防止覆盖旧草稿。浏览器恢复 Cookie 会话，修改请求使用 CSRF token 与受控 Origin；原生 Bearer 保留。
- 完整请求、响应和路由变化见 [1.4.9 协议说明](rustdesk-1.4.9-protocol.md)。设备报告不证明账户归属；管理员核验 Peer 公钥指纹后绑定。
- 必需秘密：独立 `API_JWT_SECRET`、base64 32 字节 `API_OAUTH_CONFIG_KEY`，新部署显式设置 bootstrap 管理员；不提供可上线默认密码/秘密。
- 浏览器配置：精确 `API_ALLOWED_ORIGINS`，默认安全 Cookie；本机 HTTP 例外 `API_ALLOW_INSECURE_LOCAL_HTTP=1`，显式跨站 `API_COOKIE_CROSS_SITE=1`。第三方 Cookie 限制仍存在，同源代理是可用部署方案。
- OAuth：固定 redirect URI；Google/通用 OIDC 强制完整 issuer/JWKS 和 ID token；环境 provider 只读，数据库 provider 使用稳定身份命名空间和加密 secret。
- WebSocket：`WS_TRUSTED_PROXIES` 默认空，最多 128 个精确 IPv4/IPv6 代理地址；只读取继承的进程环境，Compose/Kubernetes 已转发，无 CLI/INI 别名。代理必须覆盖转发头；错误配置拒绝启动，错误/重复/冲突头保持真实 TCP 来源。
- 所有变量与约束见 [配置参考](environment-variables.md)。自有 users/atty 薄层审计边界、实现来源及移除条件见依赖文档；不通过隐藏旧实现消除公告。

## 迁移、回滚与发布门禁

生产步骤尚未执行。上线前应：

1. 停止 hbbs/API 等写入者，进行 SQLite 一致性备份，保存匹配程序版本、RustDesk 密钥、OAuth 配置加密密钥和部署配置。
2. 在隔离备份副本上执行 integrity_check、初始化迁移与协议预检。迁移冲突整笔回滚并报告，不能丢弃数据后声称无损。
3. 核对各账户 GUID/revision、条目/标签/扩展字段及可重建索引；实机门禁通过后，才在维护窗口执行生产迁移。
4. 回滚停止写入，恢复**匹配的程序版本与数据库备份**及其密钥；禁止仅回退镜像后继续使用已迁移数据库。

详细步骤见 [地址簿存储与迁移](address-book-storage.md)、[部署与初始化](deployment.md)。本轮新库/历史迁移只使用临时数据库。

发布前用未修改的 Windows/Linux 官方 RustDesk 1.4.9 验证：密码/OAuth/撤销；地址簿初始化、分页、标签颜色、保存的连接信息、强制中继；报告和管理员绑定；两客户端注册、直连、强制中继、文件传输、重连；API 未登录时 hbbs/hbbr 连接规则。实机结果尚未取得。

## 数据与工作区保护

- 用户 `test.sqlite3` SHA-256 始终为 `2b4fba82e84e89de33d16a71df3739e24e67204b12bf62b0de12148d211b9a6e`，与基线一致。
- 最终 Cargo.lock SHA-256：`42722c2c52ceeffc095a87be61d0c3d90ff29bd3968ca3547c38a0365cdf8c52`。锁文件只随已验收依赖修复更新。
- hbb_common 提交 `f94e3fef6c962814e71b7547848e06526b235062`，源码和 gitlink 未修改。
- 没有新增生产 unwrap()/expect() 或无关全仓格式化；采用明确暂存清单，每项提交核验。构建、审计缓存和 Web 成品属于已解释的验证产物；不清理其他人的资源或数据。
