param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-flight-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/operations_test.gd -- "--flight-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Airborne save failed' }
& $Godot --headless --path $project --script res://tests/operations_test.gd -- "--flight-save=$save" --restore-flight
if ($LASTEXITCODE -ne 0) { throw 'Airborne restore failed' }
Write-Output 'PASS: airborne save and continuation across two separate Godot processes'
