param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-finance-archive-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/finances_test.gd -- "--finance-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Archive save failed' }
& $Godot --headless --path $project --script res://tests/finances_test.gd -- "--finance-save=$save" --restore-finance
if ($LASTEXITCODE -ne 0) { throw 'Archive restore failed' }
Write-Output 'PASS: complete finance archive survives separate Godot processes'
