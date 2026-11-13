param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-maintenance-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/maintenance_test.gd -- "--maintenance-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Maintenance save failed' }
& $Godot --headless --path $project --script res://tests/maintenance_test.gd -- "--maintenance-save=$save" --restore-maintenance
if ($LASTEXITCODE -ne 0) { throw 'Maintenance restore failed' }
Remove-Item $save -ErrorAction SilentlyContinue
Write-Output 'PASS: mid-maintenance save and continuation across two separate Godot processes'
