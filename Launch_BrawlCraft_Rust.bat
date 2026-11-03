@echo off
cd /d "%~dp0"
if exist "Skinner.exe" (
    start "" "%~dp0Skinner.exe"
    exit
)
if exist "publish\Skinner.exe" (
    start "" "%~dp0publish\Skinner.exe"
    exit
)
if exist "target\release\Skinner.exe" (
    start "" "%~dp0target\release\Skinner.exe"
    exit
)
if exist "BrawlCraft.exe" (
    start "" "%~dp0BrawlCraft.exe"
    exit
)
cargo run --release
