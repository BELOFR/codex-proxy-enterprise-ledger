param([ValidateSet('docker','native')][string]$BuildMode='docker')
$ErrorActionPreference='Stop'
$ledgerBuildRoot=Split-Path -Parent $PSScriptRoot
Push-Location $ledgerBuildRoot
try {
    & pnpm --dir frontend install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw '前端依赖安装失败' }
    & pnpm --dir frontend lint
    if ($LASTEXITCODE -ne 0) { throw '前端检查失败' }
    & pnpm --dir frontend build
    if ($LASTEXITCODE -ne 0) { throw '前端构建失败' }
    if ($BuildMode -eq 'docker') {
        & docker info --format '{{.ServerVersion}}'
        if ($LASTEXITCODE -ne 0) { throw 'Docker 未运行，请由操作者启动后重试' }
        & docker run --rm --platform linux/amd64 --workdir /work --mount "type=bind,src=$ledgerBuildRoot,dst=/work" -e CARGO_TARGET_DIR=/work/backend/target-linux rust:1.99.1-bookworm bash -c 'rustup component add rustfmt clippy && bash scripts/package-linux.sh'
    } else {
        if ($IsWindows) { throw 'native 模式须在 Linux x86_64 运行' }
        & bash scripts/package-linux.sh
    }
    if ($LASTEXITCODE -ne 0) { throw 'Linux 构建或插件打包失败' }
} finally { Pop-Location }
