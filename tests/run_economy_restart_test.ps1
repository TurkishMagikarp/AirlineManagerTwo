param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-economy-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/economy_test.gd -- "--economy-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Financial save failed' }
& $Godot --headless --path $project --script res://tests/economy_test.gd -- "--economy-save=$save" --restore-economy
if ($LASTEXITCODE -ne 0) { throw 'Financial restore failed' }
Write-Output 'PASS: financial results and balances survive separate Godot processes'
