param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-insurance-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Isolated explicit paths; each writer exits before its fresh reader starts.
foreach ($version in @('v22', 'v23')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/insurance_test.gd -- "--insurance-mode=$mode" "--insurance-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Insurance restart failed: $mode" }
    }
}
Write-Output "PASS: v22 neutral migration and v23 mid-repair player/AI insurance continuation in four separate processes; test files: $directory"
