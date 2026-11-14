param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-maintenance-bases-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
foreach ($version in @('v28', 'v29')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/maintenance_bases_test.gd -- "--bases-mode=$mode" "--bases-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Maintenance bases restart failed: $mode" }
    }
}
Write-Output "PASS: v28 migration and v29 running outstation checks; full player/AI worlds and APIs after load and seven days in four separate processes; files: $directory"
