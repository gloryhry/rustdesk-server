# 四项修复与原版 Linux 1.4.9 验收

本轮基线为 `40e9a132c80b7dd0fbc5aace98e6603e1d877965`，验收阶段在 `codex/rustdesk-client-compat-fixes` 工作分支修改，未执行生产迁移或发布。验收日期为 2026-10-01；运行标识 `01a0f549`。历史 155 项 Rust / 15 项浏览器结果仍保留在旧台账，本报告记录本轮新增工作。

## 修复与回归

| 项目 | 实现 | 验收结果与回归 |
| --- | --- | --- |
| A：关闭 API 的 s6 启动 | 新增互斥的 `--initialize-keys`；入口保留 flock 并按 API 开关分流；两份健康检查条件化 | 通过。无需 API 秘密/Web/数据库的密钥初始化、重复初始化不轮换、部分/损坏/不匹配密钥拒绝；全新卷 core-only 健康、重启哈希不变；启用 API 的配置与 readiness 负例继续通过 |
| B：稳定 ID 与 Peer ID 碰撞误删 | 完整扫描稳定 ID 后再兼容 Peer ID，两条 Web 删除路由使用同一逻辑 | 通过。两种顺序与两条路由的碰撞回归、旧 Peer ID、快照删除、账户隔离、revision 和存储回滚；真实浏览器删除 B、刷新后 A 保留 |
| C：设备名称搜索不一致 | SQLite CTE 派生有效名称，DTO、COUNT 和分页使用同一值 | 通过。实际 UDP 注册/报告/绑定后搜索；报告覆盖、hostname、旧信息、数据库回退、null/非字符串/空串/损坏 JSON/字面量特殊字符及分页/权限回归；真实客户端报告名称 `goal-a-laptop` 按 `Goal-A` 搜索 total=1，与原生卡片相符 |
| D：清理会话丢失 OAuth 入口 | 保留公开选项；退出成功后刷新；临时查询失败保留上次结果 | 通过。两种请求完成顺序、退出后刷新/停用、刷新失败、撤销 Cookie 后 401，以及原 Cookie/CSRF/跨源/旧 Bearer 清理测试；真实 Keycloak 退出后可再次原生登录 |
| 实际联调阻断：安全 rendezvous | hbbs TCP 发送签名临时 Curve25519 公钥，接收官方交换并双向加密帧，sink 转移后保留发送加密状态 | 通过。旧实现真实客户端报 `Failed to secure tcp: deadline has elapsed`；新增签名验证/加密打洞/普通帧兼容、错误交换关闭的两个 TCP 回归；修复后原版客户端登录状态下连接成功 |
| 验收工具异常清理 | kind 工具无法执行时，finally 仍清理 Docker 资源 | 通过。先按失败日志精确清理遗留资源，再用实际 Compose/s6 和不存在的 kind 路径复验；部署按预期失败，专属容器/网络/卷仍全部清理 |

每项先执行失败回归，再修复。红/绿日志位于 [本轮自动化证据](acceptance/goal-01a0f549/automation/)。没有新增生产 `unwrap()`/`expect()`，没有数据库迁移或全仓格式化。最终直接检查全部变更，没有发现需要继续修复的可证实新增缺陷。

## 环境与来源

两个独立 Ubuntu 24.04 amd64 桌面容器，独立 home 卷、网络地址与机器身份，Xvfb/Openbox/D-Bus，以及真实 elogind/PAM seat0 X11 活动用户会话。noVNC 仅发布到宿主机回环地址，实际操作经原版客户端界面完成。环境构建和操作步骤见 [原生验收工具说明](../tests/native/README.md)。

本机同名 1.4.9 包实际为定制版，缺少自定义服务入口；因此仅在测试容器改用 [官方 GitHub 1.4.9 amd64 包](https://github.com/rustdesk/rustdesk/releases/download/1.4.9/rustdesk-1.4.9-x86_64.deb)。SHA-256 与 GitHub release asset digest 相符：`7244ba47c40e804172044bfbe659467c54ce46554c98e78c8c0406f1d612fda3`。两端程序和共享库均未经修改，执行文件 SHA-256 均为 `5677c42b7561f2d4b9e5d8561964a92b5946f0ffa6a56b35a241c5809986c023`。

Keycloak 使用固定版本 26.4.0、专用 realm 与 PKCE S256 应用，使用实际授权/token/JWKS 接口。`api.goal.test` 与 `idp.goal.test` 经 TLS 代理，测试 CA 仅加入测试容器和专用浏览器配置；未关闭证书校验，Web 使用 Secure Cookie。随机秘密写入 0600 文件，未使用现有数据库、密钥或宿主机客户端配置。

A ID `288596091`，B ID `512499333`。管理员绑定前从各客户端本地公钥独立计算指纹；身份、公钥指纹及绑定记录见 [绑定与搜索证据](acceptance/goal-01a0f549/native/device-binding-search.json)。桌面缺少活动 console session 时，原版客户端拒绝文件传输；补齐真实 elogind/PAM 后完成传输，没有修改客户端或模拟协议。

服务镜像从修复后的源码使用 Dockerfile 默认 release 构建 runtime、supervisor 两目标。执行镜像 ID、二进制 SHA-256、客户端库哈希、工具版本及生产 diff 哈希见 [版本清单](acceptance/goal-01a0f549/native/provenance.json)。真实联调时 hbbr/API 容器沿用四项修复后的镜像，其二进制与新增握手后的镜像完全一致；hbbs 使用新构建版本。新增握手后的完整镜像另经 Compose/s6/kind 复验，core-only 使用新 supervisor 镜像。

## 真实流程结果

所有下列操作均已实际执行。API 调用仅用于准备测试夹具、独立读取结果和验证已撤销凭据；远控、键鼠及文件传输均通过原版客户端 UI 完成。

| 顺序 | 预期 | 实际结果 | 主要证据（相对于本轮 native 目录） |
| --- | --- | --- | --- |
| 1 | 服务健康、静态资源、管理员及普通用户登录 | 通过，HTTPS Web 实际登录；部署 readiness/static/login 通过 | `web-password-login.png`；automation 中部署日志 |
| 2 | A/B 配置服务地址、公钥并注册不同 ID | 通过，两个独立身份注册与 ready；最终 core 模式也记录实际注册 | `device-binding-search.json`、`s6-registrations.json`、原生日志 |
| 3 | 本地公钥指纹绑定，归属和名称搜索正确 | 通过，普通用户只见所属设备，实际名称片段 total=1 | `device-binding-search.json`；Rust `device_dto`/分页回归 |
| 4 | 原版密码登录、刷新、退出、撤销 | 通过，A/B 都实际登录；原生退出及管理员撤销后的实际令牌均返回 401 | `native-logout-token.json`、`native-revoked-token.json` |
| 5 | Web/原生经真实 Keycloak 登录，退出后再用入口 | 通过，Web 返回 goal-oidc；原生轮询领取实际身份；撤销后再次 OAuth 登录并退出，新凭据也返回 401 | `web-keycloak-login.png`、`native-keycloak-identity.json`、`native-oidc-repeat.json`、`native-oidc-logout.json` |
| 6 | 地址簿、标签、连接字段和碰撞删除跨端一致 | 通过，B 原生别名修改→Web；Web 修改→A/B 原生；RDP/loginName/hash/note 等字段保留；过期 revision 拒绝覆盖；碰撞 B 删除后 A 保留 | `address-book-native-edit.json`、`address-book-web-edit.json`、`address-book-native-{a,b}.png`、`collision-{before,after}.json`、`collision-web-{before,after}.png` |
| 7 | 实际直连画面、键鼠输入与路径 | 通过，动态时钟变化，远端程序收到 direct70101 键盘及鼠标；A/B 直接 TCP socket 与原生日志一致 | `direct-screen-{1,2}.png`、`direct-input.json`、`direct-network.txt`、`client-a-rustdesk.log` |
| 8 | 强制中继画面、键鼠与 hbbr 路径 | 通过，原版 `/r` 后缀；两端 socket→hbbr，服务器会话配对且流量增长，远端收到 relay70101 | `relay-screen-{1,2}.png`、`relay-input.json`、`relay-network.txt`、hbbr 日志 |
| 9 | 直连、中继分别双向传中文名随机二进制和空文件 | 通过，原生 GUI send/receive；每端独立随机 1 MiB `随机文件.bin` 与 `空文件.txt`，目标大小/hash 与源及初始清单完全一致 | `{direct,relay}-{send,receive}.png`、`{direct,relay}-transfer-hashes.json`、`relay-transfer-network.json` |
| 10 | 重启后 120 秒内重新注册/连接，数据保留 | 通过，12.38 秒服务就绪，73.73 秒内实际中继重连；密钥、绑定 ID、GUID/revision/内容保持；客户端日志记录 hbbs 重新请求公钥 | `restart-{before,after}.json`、`restart-connection.json`、`restart-screen.png`、原生日志 |
| 11 | 关闭 API 的 s6 仍支持真实注册/直连/中继 | 通过，无 API 秘密，healthy，21114 无监听；强制中继与 A→B 直接 TCP 都有真实画面及键鼠事件，密钥不变 | `s6-core-start.json`、`s6-registrations.json`、`s6-{direct,relay}-{screen.png,input.json,network.json}` |

完整 [native 证据目录](acceptance/goal-01a0f549/native/) 保留日志、截图、结果及资源清单；日志已脱敏，截图检查后排除了凭据。临时操作造成的 revision 冲突以及首次连接沿用强制中继设置均如实记录，未把错误路径计为直连通过。

## 自动化与审计

本轮新增 7 项 Rust、5 项浏览器回归，总计 **162 项 Rust、20 项浏览器测试全部通过**。新增安全握手后再次运行 Rust 全套、check/bins 及重新构建镜像的 Compose/s6/kind；Web 未受到握手改动影响，其本轮构建和 20 项浏览器结果仍有效。

```sh
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo test --locked --offline --all-targets
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo check --locked --offline --all-targets
CARGO_TARGET_DIR=/tmp/rustdesk-review-target CARGO_INCREMENTAL=0 cargo build --locked --offline --bins
npm --prefix web run build
RUSTDESK_API_BINARY=/tmp/rustdesk-review-target/debug/rustdesk-api npm --prefix web run test:browser
python3 tests/deployment/run.py --skip-build --image rustdesk-goal/server:01a0f549 \
  --kind /tmp/rustdesk-deployment-tools/kind --kubectl /tmp/rustdesk-deployment-tools/kubectl
```

RustSec 公告库刷新到 `9b3a3b73a7f42606494c943e95f8196e9994df46`，更新于 `2026-09-30T09:15:39+02:00`，含 1277 条公告；检查锁文件 548 个依赖，漏洞 0、unsound 0，无 ignore。npm 生产依赖漏洞 0。停止维护提示仍为 ansi_term、bincode、dlopen_derive、sodiumoxide 四项，本轮未扩展依赖替换。报告和原始审计 JSON 保留在 automation 目录。

## 清理与边界

按两次 native 环境清单及部署日志中的唯一资源名清理，本轮容器、网络、卷、kind 集群、唯一镜像标签和临时凭据已移除。未执行全局 prune，未清理其他服务、共享基础镜像或构建缓存。独立清理核验见 [cleanup.json](acceptance/goal-01a0f549/native/cleanup.json) 与部署异常清理证据。用户 `test.sqlite3` SHA-256 与基线一致：`2b4fba82e84e89de33d16a71df3739e24e67204b12bf62b0de12148d211b9a6e`。

完成结论限于四项缺陷、新增联调阻断修复和本轮原版 Linux amd64 1.4.9 隔离桌面验收。Windows、ARM、真实硬件桌面、LDAP、共享地址簿、完整 RBAC、浏览器远控等未纳入范围。依赖审计与此次流程通过不代表不存在任何未知漏洞。
