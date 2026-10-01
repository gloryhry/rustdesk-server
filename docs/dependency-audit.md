# Rust 依赖审计新增问题

基线锁文件安全审计报告包含 19 条漏洞公告；npm 生产依赖审计为 0。以下问题逐个验收、中文提交。审计结果按当前 advisory 数据库计算，不代表未来永远无漏洞。

| 新编号 | 依赖 / 公告 | 状态 |
| --- | --- | --- |
| #21 | axum-core 0.2.4 / RUSTSEC-2022-0055 | 已完成 |
| #22 | crossbeam-epoch 0.9.8 / RUSTSEC-2026-0204 | 已完成 |
| #23 | openssl 0.10.68 / RUSTSEC-2025-0022; openssl 0.10.68 / RUSTSEC-2025-0004 | 已完成 |
| #24 | remove_dir_all 0.5.3 / RUSTSEC-2023-0018 | 已完成 |
| #25 | rustls 0.23.42 / RUSTSEC-2026-0285 | 已完成 |
| #26 | tungstenite 0.17.2 / RUSTSEC-2023-0065 | 已完成 |
| #27 | ring 0.16.20 / RUSTSEC-2025-0009 | 已完成 |
| #28 | libsqlite3-sys 0.24.2 / RUSTSEC-2022-0090; sqlx 0.6.0 / RUSTSEC-2024-0363; rustls 0.20.4 / RUSTSEC-2024-0336; webpki 0.22.0 / RUSTSEC-2023-0052 | 已完成 |
| #29 | h2 0.3.26 / RUSTSEC-2026-0258; rustls-webpki 0.101.7 / RUSTSEC-2026-0098; rustls-webpki 0.101.7 / RUSTSEC-2026-0099; rustls-webpki 0.101.7 / RUSTSEC-2026-0104 | 已完成 |
| #30 | quick-xml 0.39.4 / RUSTSEC-2026-0194; quick-xml 0.39.4 / RUSTSEC-2026-0195 | 已完成 |
| #31 | users 0.11.0 / RUSTSEC-2025-0040 | 已完成 |

旧 rustls 0.20.4 / ring / webpki 路径随 SQLx 和 JWT 升级移除；旧 rustls-webpki 与 h2 需迁移 HTTP 栈。每个组件处理后重跑定向审计、根包测试与 locked check；最终审计禁止忽略公告。Wayland/users 位于 hbb_common 子模块依赖路径，修复应保持可重建来源。

修复依赖顺序：#21 → #22 → #23 → #24 → #25 → #26 → #28 → #29 → #27 → #30 → #31。ring 定向审计在旧数据库和 HTTP TLS 路径移除后才能通过，不能提前把仅更新 JWT 记成完成。

## #21 验收

`security_dependencies::oversized_json_is_rejected_before_login_and_small_requests_still_work`：红测试返回 400，升级后返回 413；正常请求与更严格 sysinfo 限制保持。根包 142 项与 locked check、API/Web 构建及 Chrome 15 项通过。

```bash
python3 tests/dependencies/audit.py --audit /path/to/cargo-audit RUSTSEC-2022-0055
```

定向脚本不忽略未修复公告，输出其他待处理数量；最终必须运行无 ignore 的完整 `cargo audit`。当前依赖为 axum 0.5.17 / axum-core 0.2.9。提取器默认 2 MiB 限制之外，服务已有更严格的 1 MiB 全局限制；本服务已存在缓解，不能把依赖公告直接等同于可在此服务无限分配内存。

## #22 验收

基线定向审计失败；仅更新 crossbeam-epoch 0.9.8 → 0.9.20 后 RUSTSEC-2026-0204 回归通过，其余 17 条仍待处理。根包 142 项测试和 locked 全目标检查通过；本项没有 API/Web 行为变化。锁文件校验为 077708abe05066773e13ee13f9c8d68ae6c6d6d06e08867799fb0736f7ba1c19，用户数据库保持基线。

## #23 验收

两条 OpenSSL 定向公告在基线失败；升级绑定 0.10.75 及必需的 openssl-sys 0.9.117 后通过，其余 15 条公告待处理。根包 142 项与 locked 全目标检查通过，服务链接和既有 OAuth/协议回归正常；没有 Web 代码变更。Cargo.lock SHA-256：4c30f9b8282a2a8183e81477bac33fd00fcbf4a1dc89d998d1c95ed64407e820，用户数据库保持基线。

## #24 验收

定向审计红测试确认 RUSTSEC-2023-0018；升级代码生成器的 tempfile 到 3.23.0 后移除 remove_dir_all 0.5.3，定向审计通过，剩余 14 条。路径位于 protobuf 构建工具，不直接接收 API 网络请求。根包 142 项测试和 locked 全目标检查通过，包括重新生成/编译协议。Cargo.lock SHA-256：d54209a0e7cb87a0a3d44b540bd5f8a97e7eb7f2721481747cb5ba068b4888cf；用户数据库未变。

## #25 验收

现代 rustls 握手边界定向审计先失败；升级 rustls 0.23.45 与所需 rustls-webpki 0.103.15 后通过，其余 13 条仍待处理。旧 rustls 0.20 的公告仍在 #28，不误报全部 TLS 已完成。根包 142 项和 locked 全目标检查通过；CryptoProvider、平台证书验证等依赖正常编译。锁文件 SHA-256：7f7341e33f2c15f510ec30c6c665ba03ce0e4b1fca8773b221dcaf9bb8ca14fb；用户数据库未变。

## #26 验收

定向 DoS 公告基线失败；tokio-tungstenite/tungstenite 统一到已由 hbb_common 使用的 0.26.2，旧 0.17 路径移除后通过，剩余 12 条。适配二进制 Bytes 类型，hbbr 发送不再复制为 Vec。真实 hbbs WebSocket 握手、RustDesk protobuf 二进制响应在升级前后都通过；保留既有 RegisterPk 在 WS 上返回 NOT_SUPPORT 的规则，未据此宣称 WS 公钥注册兼容。根包 143 项和 locked 全目标检查通过；锁文件 SHA-256 为 def89a18bbc6e2cf3a46139d889bdfe588f01287d18737a57f06780001b85c6b，用户数据库未变。

## #28 验收

四条数据库/旧 TLS 公告的定向审计基线失败；SQLx 0.8.6、libsqlite3-sys 0.30.1 升级并移除旧 rustls/webpki 路径后通过，剩余 8 条。仅启用 runtime-tokio/sqlite/derive，避免未使用的 MySQL/RSA 引入 RUSTSEC-2023-0071，该公告也纳入定向检查。适配消费型连接选项和事务连接解引用；既有 FromRow 与 SQLite 事务语义保留。ed25519 1.5.3 修正旧版本过宽 signature 依赖导致的解析编译冲突。

根包 144 项测试与 locked 全目标检查通过，覆盖历史迁移、并发启动、地址簿 CAS、失败回滚及 10,000 条 Peer 并发读写。沙箱内网络绑定测试失败后，在允许隔离回环端口的环境完整重跑通过。Cargo.lock SHA-256：2e79b9b3dd92f7927efa3dce2243f3781f8aa3f03e08ab9f6f0284632c5b1637；用户数据库保持基线。

## #29 验收

四条 HTTP/旧 TLS 公告在基线定向审计失败。Axum 0.8.9、tower-http 0.6.11、reqwest 0.12.28、http 1/tower 0.5 更新后通过；删除旧 git reqwest、hyper 0.14 测试依赖和未使用 headers，剩余 4 条。适配路径参数、middleware、监听器和静态服务，测试响应读取采用明确上限；保留两平台 TLS、代理、JSON、gzip 与 HTTP/2 支持。

真实随机回环 HTTP/1.1 与 HTTP/2 API/静态资源/413 回归通过；根包 145 项、locked 全目标检查、Web 构建与 Chrome 15 项通过。锁文件 SHA-256：0b7a50b740ca96fab736a5aa1bdc9bc166a1f6d4ee5527db509716bfa9a8b258；用户数据库保持基线。最终部署还须重建包含本项的镜像。

## #27 验收

ring 定向公告基线失败；jsonwebtoken 9.3.1 移除 ring 0.16.20，检查通过，剩余 3 条。JWK 新 key_algorithm 转换失败或不匹配签名算法均拒绝；不放宽既有 RSA 白名单。OIDC 7 项签名/算法/issuer/audience/nonce/有效期/subject/多受众攻击回归、Cookie/原生 OAuth 与根包 145 项全部通过，locked 全目标 check 通过。无 Web 代码变更；锁文件 SHA-256：f1e3e6aa32cf93cc1d8793c7215ce09159cc9115a369382359c940764f1f847e，用户数据库未变。

## #30 验收

quick-xml 两条定向公告基线失败；wayland-scanner 0.31.11 使用 quick-xml 0.41.0，升级后通过，剩余 users 一条公告。仅锁文件更新，不改 hbb_common 源码或 gitlink；Wayland 构建时 XML 代码生成重新编译通过，路径不是 API 网络输入。根包 145 项与 locked 全目标检查通过。锁文件 SHA-256：51fb8695e32a45bc41fca9968d744486ec412e19211b8e4ceb8852f308ebd72f；用户数据库未变。

## #31 验收

真实 nobody 组查询红测试：系统只返回 65534，旧 users 错误返回 0/65534。新本地 users 薄层仅重新导出维护中的 uzers 0.12.2，不包含旧实现；hbb_common 源码/gitlink 保持不变。锁文件实际实现来自 crates.io 校验包；Docker 构建明确复制兼容层。

本地包版本保持 users 0.11.0 只为兼容，cargo audit 不按旧 registry users 公告审计此本地薄层；不以公告消失作为唯一证明。系统组查询/当前 UID 正例通过，根包 146 项与 locked 全目标检查通过。当前 hbb_common 只调用 UID/home 查询，不调用受影响的组查询，未声称本服务已存在权限提升利用。锁文件 SHA-256：079b0eb61ee789359ce2d5ecd3cbb5240aaaafdd417fa4e1a3dbedcc40aa6af2，用户数据库未变。

## 完整审计新增内存安全问题

完整审计漏洞计数为 0，但还有 5 条 unsound 公告；作为独立问题继续处理。维护状态警告单独记录，不等同于已复现漏洞。

| 新编号 | 依赖 / 公告 | 状态 |
| --- | --- | --- |
| #34 | anyhow 1.0.57 / RUSTSEC-2026-0190 | 已完成 |
| #35 | atty 0.2.14 / RUSTSEC-2021-0145 | 待处理 |
| #36 | bumpalo 3.9.1 / RUSTSEC-2022-0078 | 已完成 |
| #37 | lexical-core 0.7.6 / RUSTSEC-2023-0086 | 已完成 |
| #38 | rand 0.8.5 / RUSTSEC-2026-0097 | 已完成 |

后续定向脚本与 CI 必须检查 unsound，最终完整审计使用 cargo audit --deny unsound，不忽略公告。

## #34 验收

新增定向脚本纳入 unsound，并在依赖更新前实际拒绝 RUSTSEC-2026-0190。anyhow 1.0.103 升级后通过，剩余 4 条内存安全公告；CI 完整审计改为 --deny unsound，禁止只按漏洞计数判定安全。公告是 context/downcast_mut 的借用边界错误，未在本项目发现直接 downcast_mut 调用，也未宣称动态复现 UB 或完成 Miri 验证。根包 146 项和 locked check 通过；锁文件 SHA-256：ee486f0d46727ebc0b4a64f9fb002a8fcab749770cb8e7b3b99d68fe7ba69c22；用户数据库未变。

## #36 验收

bumpalo 迭代器生命周期定向公告基线失败，锁定至 3.19.0 后通过，剩余 3 条 unsound。来源为 wasm-bindgen-backend 的目标相关代码生成路径；本项未宣称在 Linux 服务输入中动态复现 UAF，也未执行 WASM 目标构建。根包 146 项与 locked 全目标检查通过。锁文件 SHA-256：a2f4191e247e0f422b7c4a0816bacd4a8ce4eac5490e7f261af85d1a1aca7eb1；用户数据库未变。

## #37 验收

lexical-core 定向公告基线失败；其唯一路径为 deadpool 默认 config → config 0.11 → nom 5。项目只使用 managed 连接池，禁用未使用默认配置/非托管功能后整个旧路径移除，公告通过，剩余 rand/atty 两条。保留原连接池实现与参数，无临时修补旧解析器。根包 146 项与 locked check 通过，涵盖 10,000 条 Peer 并发读写、迁移、CAS 与失败回滚。锁文件 SHA-256：82c5149dc15dcf3edbc2ed12d5dc6f5a76f09794011b46956150b94e6f03b29b；用户数据库未变。

## #38 验收

rand 日志重入借用公告基线定向检查失败；0.8.6 修补版本后通过，剩余 atty 一条。hbb_common、ping 和构建/测试依赖使用同一更新，不改随机协议或密钥格式；未确认本项目日志满足公告全部重入触发条件，未声称运行时复现 UB。根包 146 项与 locked check 通过。锁文件 SHA-256：d5589b36cd6e0121e3ff661c4cd2475d52ee7dcb646180622640ae752e49dded；用户数据库未变。
