@echo off
setlocal
set "VS_VCVARS=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VS_VCVARS%" (
  echo Visual Studio Build Tools not found.
  exit /b 1
)
call "%VS_VCVARS%" >nul
if errorlevel 1 exit /b %errorlevel%
cargo build --manifest-path "%~dp0..\Cargo.toml"
