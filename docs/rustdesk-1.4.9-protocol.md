# RustDesk 1.4.9 个人地址簿协议

协议来源固定到官方客户端提交 `6c578292e8ebbbec708b76986ba8c4bc7c509747`
（标签 1.4.9），见 tests/fixtures/rustdesk-1.4.9-address-book.json。
样本由官方 ab_model.dart 的请求方法摘录，使用测试 ID/哈希；不包含真实连接密码。
Peer 与标签类型分别依据 peer_model.dart 和 common/hbbs/hbbs.dart。
Rust 测试解析字段类型、根对象/数组、分页总数、颜色整数和成功空响应，
并验证保存内容；不能只以 HTTP 200 判定兼容。

| 方法与路径 | 请求 | 响应 |
|---|---|---|
| POST /api/ab/personal | 空 body | 根对象 guid |
| POST /api/ab/settings | 空 body | max_peer_one_ab: 0（未配置条目数量上限） |
| POST /api/ab/shared/profiles | 空 body、current/pageSize | total: 0, data: [] |
| POST /api/ab/peers?ab=GUID | 空 body、current/pageSize | total/data，id 为 RustDesk ID |
| POST /api/ab/tags/GUID | 空 body | 原始标签数组，color 为无符号整数 ARGB |
| POST /api/ab/peer/add/GUID | Peer 对象，必需 id | 200 空 body |
| PUT /api/ab/peer/update/GUID | id 与需要修改的字段 | 200 空 body |
| DELETE /api/ab/peer/GUID | Peer ID 数组 | 200 空 body |
| POST /api/ab/tag/add/GUID | name/color 整数 | 200 空 body |
| PUT /api/ab/tag/rename/GUID | old/new | 200 空 body |
| PUT /api/ab/tag/update/GUID | name/color 整数 | 200 空 body |
| DELETE /api/ab/tag/GUID | 标签名称数组 | 200 空 body |

空 body 表示 Content-Length: 0，不是 JSON null。只支持个人地址簿，
不声称支持 Pro 共享能力。GUID 始终校验当前账户归属，其他账户和管理员
均不能借此访问不属于自己的地址簿。Peer 分页默认第一页，每页默认及上限 100，
按 RustDesk ID 稳定排序；筛选后计数。未知 GUID 返回 404。

部分 Peer 修改保留未传字段和未知扩展；显式空字符串/数组可清空。
内部 entryId 不通过新版官方 Peer DTO 返回，也不能由官方修改接口指定。
重复新增及标签重命名冲突返回 409，更新不存在条目返回 404。
数组删除为一次事务，重复删除幂等；任何无效数组元素均拒绝整笔请求。
颜色必须处于 0..4294967295，forceAlwaysRelay 输出官方字符串 true/false。
文档大小仍有 2 MiB 限制，即使 max_peer_one_ab 为 0。

保留旧版 GET/POST /api/ab 整本读写；GET.data 是 JSON 字符串，
tag_colors 仍是字符串包装的整数映射。Bearer 整本上传为整体替换，成功
返回 200 空 body；GUID 保持稳定。新旧客户端交替读取使用同一份文档。

Web 管理路由已迁移：

| 原管理路由 | 新路由 |
|---|---|
| GET /api/ab/peers | GET /api/web/ab/entries |
| POST /api/ab/peer | POST /api/web/ab/entries |
| DELETE /api/ab/peer/ENTRY_ID | DELETE /api/web/ab/entries/ENTRY_ID |
| POST /api/ab/peer/delete | POST /api/web/ab/entries/delete |
| 原批量 Peer 替换入口 | POST /api/web/ab/entries/batch |
| GET/POST /api/ab/tags | GET/POST /api/web/ab/tags |
| POST /api/ab/tags/delete | POST /api/web/ab/tags/delete |

Web DTO 使用内部条目 ID、布尔中继字段和 CSS 十六进制颜色；修改需 revision、
Cookie Origin 与 CSRF token。删除的 revision 位于查询参数。旧管理路由
不再作为条目 ID 删除别名，避免与官方书 GUID/Peer ID 数组删除混淆。
GUID/revision、版本 3 迁移和恢复备份步骤见 address-book-storage.md。

自动化验收范围仅为上述相关协议；未修改的 Windows/Linux 官方客户端实机
联调、直连/中继/文件传输/重连仍需通过发布门禁。
