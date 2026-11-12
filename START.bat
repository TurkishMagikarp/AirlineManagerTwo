@echo off
setlocal
set "GODOT_EXE="
if defined AIRLINE_GODOT (
  set "GODOT_EXE=%AIRLINE_GODOT%"
  goto validate
)
for %%G in (godot.exe godot4.exe) do (
  for /f "delims=" %%P in ('where %%G 2^>nul') do if not defined GODOT_EXE set "GODOT_EXE=%%P"
)
if not defined GODOT_EXE set "GODOT_EXE=%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe"
:validate
if not exist "%GODOT_EXE%" (
  echo FEHLER: Godot 4.6+ fehlt. AIRLINE_GODOT auf den Executable-Dateipfad setzen.
  exit /b 1
)
set "CARGO_EXE=cargo.exe"
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "CARGO_EXE=%USERPROFILE%\.cargo\bin\cargo.exe"
"%CARGO_EXE%" build --locked --manifest-path "%~dp0rust\Cargo.toml"
if errorlevel 1 (
  echo FEHLER: Rust-Build fehlgeschlagen. Rust und die C++ Build Tools muessen installiert sein.
  exit /b 1
)
rem Import registers the extension even on a fresh clone without .godot cache.
"%GODOT_EXE%" --headless --editor --path "%~dp0." --import
if errorlevel 1 exit /b 1
"%GODOT_EXE%" --path "%~dp0." %*
exit /b %ERRORLEVEL%
