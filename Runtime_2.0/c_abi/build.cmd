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
echo [1/4] Compilando tensor...
"%GCC%" %CFLAGS% "..\tensor\tensor.c" -o "%OUTDIR%\tensor.o"
if errorlevel 1 ( echo [ERROR] tensor.c & exit /b 1 )
echo   tensor.o OK

echo [2/4] Compilando memory...
"%GCC%" %CFLAGS% "..\memory\arena.c" -o "%OUTDIR%\arena.o"
if errorlevel 1 ( echo [ERROR] arena.c & exit /b 1 )
echo   arena.o OK

echo [3/4] Compilando math_ops...
"%GCC%" %CFLAGS% -I.. "..\math_ops\nn_ops.c" -o "%OUTDIR%\nn_ops.o"
if errorlevel 1 ( echo [ERROR] nn_ops.c & exit /b 1 )
echo   nn_ops.o OK

echo [4/4] Compilando nn...
"%GCC%" %CFLAGS% -I.. "..\nn\linear.c" -o "%OUTDIR%\linear.o"
if errorlevel 1 ( echo [ERROR] linear.c & exit /b 1 )
echo   linear.o OK

echo.
echo [Runtime 2.0] Build completo — 4/4 .o generados en %OUTDIR%
echo.

REM Compilar test_tensor
echo [BONUS] Compilando test_tensor...
"%GCC%" -O2 -mavx2 -mfma -Wall -I.. "..\tensor\test_tensor.c" "..\tensor\tensor.c" -o "%OUTDIR%\test_tensor.exe" -lm
if errorlevel 1 ( echo [WARN] test_tensor no compiló ) else ( echo   test_tensor.exe OK )

echo.
echo ======================================================
echo   Build OK — Runtime 2.0 listo
echo ======================================================
