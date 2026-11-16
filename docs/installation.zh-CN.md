# 安装、导入与升级

[返回 README](../README.md) · [English](installation.en.md) · [安装后的使用流程](usage.zh-CN.md)

本项目安装到 **Codex Proxy RS 的插件系统**。你在电脑浏览器里操作管理界面，但插件进程运行在网关所在的 Linux 服务器或容器中。它不是 Chrome 扩展，也不是 Excel 加载项。

## 1. 确认网关环境

| 项目 | 要求 |
| --- | --- |
| 宿主 | Codex Proxy RS >=3.49.0、<4.0.0，具备插件管理入口 |
| 服务器架构 | Linux x86_64 / amd64 |
| 运行库 | glibc >=2.34，libgcc_s；本次在 Debian 12 下验证 |
| 操作账号 | 宿主管理员，可安装并信任插件 |
| 网络 | 下载 Release 时需要访问 GitHub；上传文件后插件页面本身不依赖外部字体或图标服务 |

在宿主运行环境中检查架构与 glibc：

```bash
uname -m
ldd --version
```

第一项应为 `x86_64`。Alpine 通常使用 musl，不能直接运行此 glibc 包；ARM64 和 Windows 也需要另外构建对应平台版本。如果网关运行在 Docker 中，应检查**网关容器内部**的平台和运行库，而不是只看宿主机。仅安装这个插件无需在你的桌面电脑启用 Docker。

## 2. 下载正确的文件

打开 [最新 Release](https://github.com/BELOFR/codex-proxy-enterprise-ledger/releases/latest)，展开 **Assets**：

| 文件 | 用途 |
| --- | --- |
| `belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz` | **上传到网关的插件安装包** |
| 同名 `.tar.gz.sha256` | 安装包完整性校验 |
| `enterprise-ledger-0.2.0-source.zip` 及 `.sha256` | 开发源码，不用于安装 |
| GitHub 自动生成的 `Source code (zip / tar.gz)` | Git 源码快照，不用于安装 |

不要解压安装包，不要改名成 ZIP，也不要将源码压缩包上传到插件管理。

## 3. 校验下载

Linux / macOS 在两个文件所在目录运行：

```bash
sha256sum -c belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
# macOS 没有 sha256sum 时：
shasum -a 256 belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz
```

`sha256sum -c` 应返回 `OK`。使用 `shasum` 时，将输出与 `.sha256` 的第一段64位摘要比较。

Windows PowerShell：

```powershell
Get-FileHash .\belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz -Algorithm SHA256
Get-Content .\belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

两处摘要应相同，不区分大小写。不同则重新下载，不要继续安装。

## 4. 在网关导入插件

1. 用管理员账号登录 Codex Proxy RS 管理端。
2. 打开侧边栏中的「插件管理」，选择「安装插件」。
3. 安装方式选择「上传包」，浏览并选中下载的 `.tar.gz`。浏览器在 Windows 上也可以上传 Linux 包；运行平台由服务器决定。
4. 点击包校验／检查操作，等宿主完成解析。实际按钮文案可能随宿主语言和版本略有不同。
5. 查看检查结果，确认发布者 `belofr`、插件 ID `belofr.enterprise-ledger`、版本 `0.2.0`、目标平台 Linux x64，以及你下载的来源。
6. 阅读宿主的完整信任说明，再确认安装。原生插件进程以宿主的操作系统身份运行，因此应只安装你确认可信的源码和包。
7. 安装完成后在插件详情检查状态。默认配置为空对象，台账信息在插件内部配置；如果宿主未自动启用，请在插件管理中启用。
8. 从该插件的管理页面入口打开「企业用量与费用台账」。看到带书本图标的侧边导航后，使用右上角「中文 / EN」选择语言。

你也可能看到宿主提供「GitHub」或「下载地址」安装方式；本指南采用可手动核对包和摘要的上传方式。宿主版本发生变化时，以当前页面校验结果为准。

## 5. 安装后先做什么

进入「使用指南」，按「人员归属 → 费用池 → 付款 → 预算 → 查询与导出」逐步配置，详见 [完整使用流程](usage.zh-CN.md)。

第一次可以先查询最近一天的用量：尚未登记员工时，请求会显示「未归属」；尚未登记付款时，不会自动推断订阅价格或实际付款。

## 升级与数据保留

1. 先导出重要月份的 Excel，并按公司的备份流程备份宿主数据。
2. 下载新版本同一插件 ID 的安装包与摘要，重复校验和上传流程。
3. 在宿主中为已安装插件选择安装新版本／更新，核对版本与信任结果。通常不需要先卸载。
4. 更新完成后，检查人员归属、付款、预算及月结列表，重新生成一次统计。

`0.2.0` 沿用 `0.1.0` 的插件 ID 与状态格式，界面语言不会改写财务数据。宿主自身的备份、卸载和数据删除行为仍以其实现为准；不要把卸载当成常规升级步骤。

## 常见问题

| 现象 | 排查方法 |
| --- | --- |
| 找不到插件管理 | 核对宿主版本与管理员权限；这是 Codex Proxy RS 插件，不在浏览器扩展商店安装 |
| 包解析失败 | 确认上传的是 `belofr.…linux-gnu.tar.gz`，而不是源码 ZIP／自动生成的 Source code |
| 平台不匹配或无法执行 | 检查网关容器内部是否为 Linux x86_64；检查 glibc >=2.34 及 libgcc_s，排除 musl/ARM |
| 安装成功但没有页面 | 查看插件是否已启用、管理能力是否注册成功以及宿主插件日志，再从该插件的管理页面入口打开 |
| 页面提示从宿主打开 | 不要直接打开导出的 `index.html`；页面需要宿主提供请求和下载桥 |
| 统计为空 | 先选真实有请求的日期，检查报表时区、宿主明细保留期以及用量采集状态 |
| Excel 下载无响应 | 先生成文件，再取得链接；HTTP 环境复制一次性地址到新标签页，HTTPS 环境按宿主链接确认提示操作 |
| 链接过期或已使用 | 返回报表重新生成 Excel，再取得新链接。一个链接只可成功交付一次 |
| 操作时提示版本冲突 | 先重新加载配置，核对其他管理员已保存的内容后再操作，避免自动重试付款写入 |

当前兼容性和验收范围见 [验证记录](verification.md)。提交问题时请附宿主与插件版本、运行平台、复现步骤，隐藏真实 Key、令牌及财务记录。
