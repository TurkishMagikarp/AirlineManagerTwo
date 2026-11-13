param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-competition-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/competition_test.gd -- "--competition-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Competition save/test failed' }
& $Godot --headless --path $project --script res://tests/competition_test.gd -- "--competition-save=$save" --restore-competition
if ($LASTEXITCODE -ne 0) { throw 'Competition fresh-process restore failed' }
Write-Output 'PASS: multi-airline simulation survives separate Godot processes'
