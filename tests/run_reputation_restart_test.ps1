param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-reputation-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
foreach ($version in @('v27', 'v29')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/reputation_test.gd -- "--reputation-mode=$mode" "--reputation-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Reputation restart failed: $mode" }
    }
}
Write-Output "PASS: v27 neutral migration and v29 reputation continuation, full player/AI worlds and APIs after load and seven days in four separate processes; files: $directory"
