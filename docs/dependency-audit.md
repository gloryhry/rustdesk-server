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
| #27 | ring 0.16.20 / RUSTSEC-2025-0009 | 待处理 |
| #28 | libsqlite3-sys 0.24.2 / RUSTSEC-2022-0090; sqlx 0.6.0 / RUSTSEC-2024-0363; rustls 0.20.4 / RUSTSEC-2024-0336; webpki 0.22.0 / RUSTSEC-2023-0052 | 已完成 |
| #29 | h2 0.3.26 / RUSTSEC-2026-0258; rustls-webpki 0.101.7 / RUSTSEC-2026-0098; rustls-webpki 0.101.7 / RUSTSEC-2026-0099; rustls-webpki 0.101.7 / RUSTSEC-2026-0104 | 待处理 |
| #30 | quick-xml 0.39.4 / RUSTSEC-2026-0194; quick-xml 0.39.4 / RUSTSEC-2026-0195 | 待处理 |
| #31 | users 0.11.0 / RUSTSEC-2025-0040 | 待处理 |

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
