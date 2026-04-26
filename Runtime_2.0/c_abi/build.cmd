@echo off
REM ============================================================
REM PyDead-BIB Runtime 2.0 — C ABI Build (temporal hasta C propio)
REM Usa MSYS2 GCC — C:\msys64\mingw64\bin\gcc.exe
REM ============================================================

set GCC=C:\msys64\mingw64\bin\gcc.exe
set CFLAGS=-O2 -mavx2 -mfma -Wall -c
set OUTDIR=..\build

echo ======================================================
echo   PyDead-BIB Runtime 2.0 — C ABI Build
echo ======================================================

if exist "%GCC%" (
    echo [OK] GCC: %GCC%
) else (
    echo [ERROR] GCC no encontrado: %GCC%
    echo Instalar: pacman -S mingw-w64-x86_64-gcc
    exit /b 1
)

if not exist "%OUTDIR%" mkdir "%OUTDIR%"

echo.
echo [1/7] Compilando tensor...
"%GCC%" %CFLAGS% "..\tensor\tensor.c" -o "%OUTDIR%\tensor.o"
if errorlevel 1 ( echo [ERROR] tensor.c & exit /b 1 )
echo   tensor.o OK

echo [2/7] Compilando memory...
"%GCC%" %CFLAGS% "..\memory\arena.c" -o "%OUTDIR%\arena.o"
if errorlevel 1 ( echo [ERROR] arena.c & exit /b 1 )
echo   arena.o OK

echo [3/7] Compilando math_ops...
"%GCC%" %CFLAGS% -I.. "..\math_ops\nn_ops.c" -o "%OUTDIR%\nn_ops.o"
if errorlevel 1 ( echo [ERROR] nn_ops.c & exit /b 1 )
echo   nn_ops.o OK

echo [4/7] Compilando nn...
"%GCC%" %CFLAGS% -I.. "..\nn\linear.c" -o "%OUTDIR%\linear.o"
if errorlevel 1 ( echo [ERROR] linear.c & exit /b 1 )
echo   linear.o OK

echo [5/7] Compilando autograd...
"%GCC%" %CFLAGS% -I.. "..\autograd\autograd.c" -o "%OUTDIR%\autograd.o"
if errorlevel 1 ( echo [ERROR] autograd.c & exit /b 1 )
echo   autograd.o OK

echo [6/7] Compilando optim...
"%GCC%" %CFLAGS% -I.. "..\optim\optim.c" -o "%OUTDIR%\optim.o"
if errorlevel 1 ( echo [ERROR] optim.c & exit /b 1 )
echo   optim.o OK

echo [7/7] Compilando io...
"%GCC%" %CFLAGS% -I.. "..\io\tensor_io.c" -o "%OUTDIR%\tensor_io.o"
if errorlevel 1 ( echo [ERROR] tensor_io.c & exit /b 1 )
echo   tensor_io.o OK

echo.
echo [Runtime 2.0] Build completo — 7/7 .o generados en %OUTDIR%
echo.

REM Compilar test_tensor
echo [BONUS 1] Compilando test_tensor...
"%GCC%" -O2 -mavx2 -mfma -Wall -I.. "..\tensor\test_tensor.c" "..\tensor\tensor.c" -o "%OUTDIR%\test_tensor.exe" -lm
if errorlevel 1 ( echo [WARN] test_tensor no compilo ) else ( echo   test_tensor.exe OK )

REM Compilar test_full (integration test)
echo [BONUS 2] Compilando test_full...
"%GCC%" -O2 -mavx2 -mfma -Wall -I.. "..\tests\test_full.c" "..\tensor\tensor.c" "..\math_ops\nn_ops.c" "..\memory\arena.c" "..\nn\linear.c" "..\autograd\autograd.c" "..\optim\optim.c" "..\io\tensor_io.c" -o "%OUTDIR%\test_full.exe" -lm
if errorlevel 1 ( echo [WARN] test_full no compilo ) else ( echo   test_full.exe OK )

echo.
echo ======================================================
echo   Build OK — Runtime 2.0 listo
echo ======================================================
