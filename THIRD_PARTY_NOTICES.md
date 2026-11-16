# 第三方组件

插件依赖及精确版本分别记录在 `backend/Cargo.lock` 与 `frontend/pnpm-lock.yaml`，依赖的许可证不因本项目分发而变更

- Codex Proxy RS 插件 SDK、官方打包工具：[zyycn/codex-proxy-rs](https://github.com/zyycn/codex-proxy-rs)，Apache-2.0，使用固定提交
- 管理页面组件及主题：[zyycn/codex-proxy-ui](https://github.com/zyycn/codex-proxy-ui)，Apache-2.0，使用 v0.8.0 对应的锁定源码
- Vue：MIT
- rust_xlsxwriter：MIT，生成原生 xlsx
- 其他 Rust 与前端依赖的作者及许可见锁定版本各自的包声明

页面构建产物包含组件库及其依赖，保留构建器生成的合法注释及相关许可标记
