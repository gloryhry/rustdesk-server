# Rust 依赖审计新增问题

基线锁文件安全审计报告包含 19 条漏洞公告；npm 生产依赖审计为 0。以下问题逐个验收、中文提交。审计结果按当前 advisory 数据库计算，不代表未来永远无漏洞。

| 新编号 | 依赖 / 公告 | 状态 |
| --- | --- | --- |
| #21 | axum-core 0.2.4 / RUSTSEC-2022-0055 | 已完成 |
| #22 | crossbeam-epoch 0.9.8 / RUSTSEC-2026-0204 | 待处理 |
| #23 | openssl 0.10.68 / RUSTSEC-2025-0022; openssl 0.10.68 / RUSTSEC-2025-0004 | 待处理 |
| #24 | remove_dir_all 0.5.3 / RUSTSEC-2023-0018 | 待处理 |
| #25 | rustls 0.23.42 / RUSTSEC-2026-0285 | 待处理 |
| #26 | tungstenite 0.17.2 / RUSTSEC-2023-0065 | 待处理 |
| #27 | ring 0.16.20 / RUSTSEC-2025-0009 | 待处理 |
| #28 | libsqlite3-sys 0.24.2 / RUSTSEC-2022-0090; sqlx 0.6.0 / RUSTSEC-2024-0363; rustls 0.20.4 / RUSTSEC-2024-0336; webpki 0.22.0 / RUSTSEC-2023-0052 | 待处理 |
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
