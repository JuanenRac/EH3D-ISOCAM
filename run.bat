@echo off
rem Runs eh3d-isocam with its sample. Extra arguments replace the sample: run.bat [ARGUMENTS]
cd /d "%~dp0"
set "ARGS=%*"
if "%ARGS%"=="" set "ARGS=isolate examples/sample.gtl examples/sample.nc --passes 2"
cargo run --release --quiet -- %ARGS%
if not defined NOPAUSE pause
exit /b %ERRORLEVEL%
