$ErrorActionPreference = 'Stop'
$ledgerRoot = Split-Path -Parent $PSScriptRoot
$env:RUSTUP_HOME = Join-Path $ledgerRoot '.tools\rustup'
$env:CARGO_HOME = Join-Path $ledgerRoot '.tools\cargo'
$env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
$ledgerVsWhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path -LiteralPath $ledgerVsWhere) {
    $ledgerVsPath = & $ledgerVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($ledgerVsPath) {
        $ledgerVsBatch = Join-Path $ledgerVsPath 'VC\Auxiliary\Build\vcvars64.bat'
        $ledgerVsEnvironment = & cmd.exe /c "call `"$ledgerVsBatch`" >nul && set"
        foreach ($ledgerEnvLine in $ledgerVsEnvironment) {
            if ($ledgerEnvLine -match '^([^=]+)=(.*)$') {
                [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
            }
        }
    }
}
$env:RUST_MIN_STACK = '16777216'
