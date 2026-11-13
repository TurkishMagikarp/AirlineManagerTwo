param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-financing-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/financing_test.gd -- "--financing-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Financing save failed' }
& $Godot --headless --path $project --script res://tests/financing_test.gd -- "--financing-save=$save" --restore-financing
if ($LASTEXITCODE -ne 0) { throw 'Financing restore failed' }
Remove-Item $save -ErrorAction SilentlyContinue
Write-Output 'PASS: loan, early repayment and exact payment across separate Godot processes'
