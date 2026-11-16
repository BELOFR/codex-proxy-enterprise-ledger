# Enterprise Ledger Implementation Plan

**Goal:** 实现可安装的 Linux x64 企业用量及费用台账插件，提供真实用量读取、组织归属、付款、费用分摊、预算、月结和 Excel

**Architecture:** 独立插件依赖固定提交的公开 Rust SDK，管理页面使用 Vue 与原项目组件。单次完整用量扫描生成一致的明细和汇总，配置与月结快照使用宿主私有状态及版本检查保存

**Tech Stack:** Rust 1.99.1、Tokio、rust_decimal、rust_xlsxwriter、Vue、TypeScript、Vite

## 约束

- 用户最新授权允许使用 Docker；采用 Windows 原生验证及 Docker Linux x86_64 构建和运行验证
- 独立插件项目，不修改宿主项目；开源仓库使用 main 主分支
- 只开发与打包，不安装至生产、不发布远端、不调用收费模型
- 沿用已确认设计，真实身份权限仍由宿主管理员入口控制

## 实施步骤

1. `backend/src/data.rs`：配置、有效期归属、付款、费用池、预算及校验；测试重复付款、跨期与重叠映射
2. `backend/src/report/`：完整扫描、请求资格、十进制金额分摊与未分摊；测试零值、缺失、多币种、退款、尾差及各维度对账
3. `backend/src/export.rs`：白名单字段原生 xlsx 与安全文本；测试文件内容及请求数量
4. `backend/src/management/`：管理 API、CAS 状态、月结、一次性下载回调；SDK 会话回归覆盖实际注册与数据回调
5. `frontend/src/`：统计、归属、付款、规则及月结页面，类型检查、构建与浏览器交互验证
6. `scripts/`：Docker Linux 构建和官方 CLI 打包，核对 Linux ELF、归档清单及 SHA-256
7. `README.md` 与 `docs/verification.md`：记录安装方式、操作、兼容合同、容量限制及实际验证缺口

在当前会话顺序执行，不创建子代理或另一个聊天。必要检查通过后打包，不等待额外审批
