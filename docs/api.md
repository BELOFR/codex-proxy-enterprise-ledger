# 管理 API

路由相对当前插件实例，仅使用 `window.codexProxyPlugin.request` 调用。管理入口要求宿主管理员身份，响应为插件 JSON，不使用宿主业务信封。失败响应为 `{ "error": { "message": "…" } }`

| 方法与路径 | 输入 | 结果 |
| --- | --- | --- |
| GET `api/state` | 无 | `version`、`settings`、月结目录、插件版本及记录上限 |
| GET `api/keys` | 无 | 完整 Key 摘要列表，仅ID、名称、启用状态 |
| POST `api/state` | `expectedVersion`、完整 `settings` | 校验后原子保存配置，保留月结，返回新版本 |
| POST `api/import-payments` | `expectedVersion`、UTF-8 `csv` | 整批校验及去重后保存，返回版本和导入行数 |
| POST `api/report` | `startDate`、`endDate`，结束日期包含在范围内 | 完整扫描后生成报表ID、全部汇总及前100条明细 |
| POST `api/export` | `reportId` | 以已查询或打开的同一快照生成文件，返回 `fileId` |
| POST `api/close` | `reportId`、`label`、`reason` | 检查报表配置版本，保存不可变快照并返回ID及版本 |
| POST `api/open` | `reportId`，此处为月结目录ID | 从已存快照生成短期报表ID，返回统计，不扫描宿主 |
| POST `api/delete-close` | `reportId`、`expectedVersion` | 移除已确认另行归档的版本，保留当前配置和付款 |

配置类型由 `backend/src/data.rs` 和页面表单共同维护，日期与金额约定见 README。金额及比例为十进制字符串，Token 总量为字符串，避免 JavaScript 整数精度丢失

`api/state` 的版本不存在时为 `null`，只能用于首次创建。保存、导入及移除发生并发冲突时返回409，重新加载后由操作者决定重试。保存失败或结果未知时不要自动重放财务操作

报表缓存15分钟，Excel 缓存10分钟。超期返回410，未完成扫描不生成报表ID；报表工作繁忙返回429。读取超时或宿主失败返回502，不将已收到批次作为完整结果

## 一次性下载

1. 调用 `api/export` 获取 `fileId`
2. 调用 `callbackTicket({path: "download", ttlSeconds: 120})`
3. 在返回地址保留宿主 `state` 参数，追加 `id=<fileId>`
4. 在新的用户点击中通过 `openExternal` 打开 HTTPS 地址，或由用户复制到新标签页

下载回调要求 GET，经过宿主一次性票据校验，插件另检查文件ID、时效及单次交付。响应为原始 xlsx 字节，带附件、`no-store` 和 `nosniff` 头。调用管理 API 不会直接启动下载

## 持久化

配置与所有在线月结以一个受版本保护的记录原子保存，gzip-base64 封装满足宿主256 KiB单条限制，解压前后均检查容量。插件不直接访问宿主数据库，也不写入原生用量或 Key 结算账本
