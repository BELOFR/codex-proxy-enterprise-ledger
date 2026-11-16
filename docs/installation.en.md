# Installation, import and upgrades

[README](../README.en.md) · [简体中文](installation.zh-CN.md) · [Usage guide](usage.en.md)

Install this package into **Codex Proxy RS plugin management**. The plugin process runs on the gateway’s Linux server/container; your browser only displays the management page. This is neither a browser extension nor an Excel add-in.

## 1. Check the gateway

| Item | Requirement |
| --- | --- |
| Host | Codex Proxy RS >=3.49.0, <4.0.0 with plugin management |
| Platform | Linux x86_64 / amd64 |
| Runtime | glibc >=2.34, libgcc_s; executed on Debian 12 |
| Account | Host administrator with plugin installation permission |
| Network | GitHub access for downloading; the installed interface does not need remote fonts or icon services |

Inside the environment that actually runs the gateway:

```bash
uname -m
ldd --version
```

Expect `x86_64` and a compatible glibc. This binary does not support Alpine/musl, ARM64 or Windows. If the host runs in Docker, check **inside that container**. Installing this plugin does not require Docker on the desktop computer used to upload it.

## 2. Download the package

Open [the latest Release](https://github.com/BELOFR/codex-proxy-enterprise-ledger/releases/latest) and expand **Assets**.

| Asset | Purpose |
| --- | --- |
| `belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz` | **Installable native plugin** |
| Matching `.tar.gz.sha256` | Package checksum |
| `enterprise-ledger-0.2.0-source.zip` and its checksum | Development source |
| Automatic GitHub `Source code` downloads | Git source snapshots |

Keep the plugin archive compressed. Source archives cannot be uploaded as plugin packages.

## 3. Verify SHA-256

Linux:

```bash
sha256sum -c belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Expect `OK`. On macOS use `shasum -a 256 <package.tar.gz>` and compare its digest with the `.sha256` file. On Windows:

```powershell
Get-FileHash .\belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz -Algorithm SHA256
Get-Content .\belofr.enterprise-ledger-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Digests must match, ignoring letter case. Redownload if they differ.

## 4. Upload into Codex Proxy RS

1. Sign in to the gateway management UI as an administrator.
2. Open **Plugin management → Install plugin**.
3. Select **Upload package** and choose the `.tar.gz`. A Windows browser can upload a Linux package; compatibility depends on the gateway server.
4. Run the package check/validation action. Exact wording can vary with host language and version.
5. Review publisher `belofr`, ID `belofr.enterprise-ledger`, version `0.2.0`, Linux x64 target and source.
6. Read the host’s full-trust notice, then confirm installation. Native plugin processes run with the host OS identity, so install packages whose source you trust.
7. Check the plugin’s status and enable it if the host did not do so automatically. The host-level configuration is an empty object; financial configuration lives inside the ledger.
8. Open its management page. The host may show the Chinese title “企业用量与费用台账”. Use **中文 / EN** at the top right of the plugin page.

The host may also offer GitHub/download-URL installation methods. This guide uses manual upload so that you can inspect the exact archive and checksum.

## 5. First report

Open **Getting started**, then follow **People → Cost pools → Payments → Budgets → Query/export**. Read the [complete workflow](usage.en.md).

You can query a recent day before adding configuration. Requests without ownership appear as Unassigned. Payment amounts are never inferred from subscriptions or usage prices.

## Upgrade

1. Export important period reports and back up host data using your company’s normal process.
2. Download and verify the new package with the same plugin ID.
3. Choose the installed plugin’s update/install-new-version flow in the host. Do not routinely uninstall before upgrading.
4. Review the version/trust results, then check ownership, payments, budgets and archives after the update.

Version 0.2.0 keeps the 0.1.0 plugin ID and state schema. Display-language changes do not modify accounting data. Host backup, uninstall and state-removal behavior remains host-controlled.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| No plugin management | Host version and administrator permissions; do not use the browser extension store |
| Invalid package | Upload the `belofr.…linux-gnu.tar.gz` asset, not source ZIPs |
| Cannot execute/platform mismatch | Inspect architecture and glibc **inside the gateway container**; exclude ARM and musl |
| Installed but no page | Plugin enabled status, management registration and host plugin logs |
| Page asks to open from the host | Do not open `index.html` directly; the host provides the request/download bridge |
| No records | Query a period with real requests; verify time zone, retention and usage collection |
| No Excel download | Generate the file, get a link, then open it; HTTP hosts require copying the URL into a new tab |
| Expired/already-used link | Generate another export and request a new link |
| Version conflict | Reload and inspect the latest configuration before writing again |

See [verification](verification.md) for actual coverage and limits. Include host/plugin versions, platform and reproduction steps in bug reports; redact credentials and financial data.
