@echo off
REM ============================================================
REM PyDead-BIB Runtime 2.0 — CUDA Build
REM NVCC — CUDA 13.1
REM ============================================================

set NVCC="C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1\bin\nvcc.exe"
set NVFLAGS=-O2 -arch=sm_86 --ptx

echo [Runtime 2.0] Compilando CUDA kernels...

if exist %NVCC% (
    echo [OK] NVCC encontrado
) else (
    echo [ERROR] NVCC no encontrado en %NVCC%
    exit /b 1
)

for %%f in (*.cu) do (
    echo   Compilando %%f...
    %NVCC% %NVFLAGS% "%%f" -o "%%~nf.ptx"
    if errorlevel 1 (
        echo [ERROR] Fallo en %%f
        exit /b 1
    )
)

echo [Runtime 2.0] CUDA kernels compilados OK
