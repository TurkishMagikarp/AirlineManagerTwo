param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-disruption-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Separate explicit save paths; each writer exits before its reader starts.
foreach ($version in @('v25', 'v26')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/disruptions_test.gd -- "--disruption-mode=$mode" "--disruption-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Disruption restart failed: $mode" }
    }
}
Write-Output "PASS: v25 migration and v26 mid-delay player/AI continuation in four separate processes; files: $directory"
