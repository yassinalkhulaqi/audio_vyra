#requires -Version 7
$ErrorActionPreference = 'Stop'
$env:CARGO_INCREMENTAL = '0'

function Step([string]$name, [scriptblock]$action) {
  Write-Host "`n== $name ==" -ForegroundColor Cyan
  & $action
  if ($LASTEXITCODE -ne 0) {
    Write-Host "FAILED: $name" -ForegroundColor Red
    exit $LASTEXITCODE
  }
}

Step 'format' { cargo fmt --all -- --check }
Step 'clippy' { cargo clippy --workspace --all-targets -- -D warnings }
Step 'tests' { cargo test --workspace }
Step 'build' { cargo build --workspace }

Write-Host "`nAll checks passed." -ForegroundColor Green
