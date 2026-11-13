param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$save = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-staff-' + [guid]::NewGuid() + '.json')
& $Godot --headless --path $project --script res://tests/staff_test.gd -- "--staff-save=$save"
if ($LASTEXITCODE -ne 0) { throw 'Staff save failed' }
& $Godot --headless --path $project --script res://tests/staff_test.gd -- "--staff-save=$save" --restore-staff
if ($LASTEXITCODE -ne 0) { throw 'Staff restore failed' }
Remove-Item $save -ErrorAction SilentlyContinue
Write-Output 'PASS: staff, salaries and a waiting check across two separate Godot processes'
