@echo off
REM ============================================================
REM PyDead-BIB Runtime 2.0 — MSVC Build (VS 2022 Build Tools)
REM cl.exe — C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools
REM ============================================================

set VCVARS="C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
set OUTDIR=..\build_msvc

echo ======================================================
echo   PyDead-BIB Runtime 2.0 — MSVC Build
echo ======================================================

if exist %VCVARS% (
    echo [OK] vcvarsall.bat encontrado
) else (
    echo [ERROR] vcvarsall.bat no encontrado en %VCVARS%
    echo Instalar: VS 2022 Build Tools con workload VCTools
    exit /b 1
)

REM Inicializar entorno MSVC x64
call %VCVARS% x64 >nul 2>&1
if errorlevel 1 (
    echo [ERROR] vcvarsall.bat fallo
    exit /b 1
)
echo [OK] MSVC x64 environment initialized

if not exist "%OUTDIR%" mkdir "%OUTDIR%"

set CFLAGS=/O2 /arch:AVX2 /W3 /c /nologo /D_CRT_SECURE_NO_WARNINGS

echo.
echo [1/7] Compilando tensor...
cl %CFLAGS% /I.. "..\tensor\tensor.c" /Fo"%OUTDIR%\tensor.obj"
if errorlevel 1 ( echo [ERROR] tensor.c & exit /b 1 )
echo   tensor.obj OK

echo [2/7] Compilando memory...
cl %CFLAGS% /I.. "..\memory\arena.c" /Fo"%OUTDIR%\arena.obj"
if errorlevel 1 ( echo [ERROR] arena.c & exit /b 1 )
echo   arena.obj OK

echo [3/7] Compilando math_ops...
cl %CFLAGS% /I.. "..\math_ops\nn_ops.c" /Fo"%OUTDIR%\nn_ops.obj"
if errorlevel 1 ( echo [ERROR] nn_ops.c & exit /b 1 )
echo   nn_ops.obj OK

echo [4/7] Compilando nn...
cl %CFLAGS% /I.. "..\nn\linear.c" /Fo"%OUTDIR%\linear.obj"
if errorlevel 1 ( echo [ERROR] linear.c & exit /b 1 )
echo   linear.obj OK

echo [5/7] Compilando autograd...
cl %CFLAGS% /I.. "..\autograd\autograd.c" /Fo"%OUTDIR%\autograd.obj"
if errorlevel 1 ( echo [ERROR] autograd.c & exit /b 1 )
echo   autograd.obj OK

echo [6/7] Compilando optim...
cl %CFLAGS% /I.. "..\optim\optim.c" /Fo"%OUTDIR%\optim.obj"
if errorlevel 1 ( echo [ERROR] optim.c & exit /b 1 )
echo   optim.obj OK

echo [7/7] Compilando io...
cl %CFLAGS% /I.. "..\io\tensor_io.c" /Fo"%OUTDIR%\tensor_io.obj"
if errorlevel 1 ( echo [ERROR] tensor_io.c & exit /b 1 )
echo   tensor_io.obj OK

echo.
echo [Runtime 2.0] Build completo — 7/7 .obj generados en %OUTDIR%
echo.

REM Compilar test_tensor
echo [BONUS 1] Compilando test_tensor.exe...
cl %CFLAGS% /I.. "..\tensor\test_tensor.c" /Fo"%OUTDIR%\tt_main.obj"
cl %CFLAGS% /I.. "..\tensor\tensor.c" /Fo"%OUTDIR%\tt_tensor.obj"
link /nologo "%OUTDIR%\tt_main.obj" "%OUTDIR%\tt_tensor.obj" /OUT:"%OUTDIR%\test_tensor.exe"
if errorlevel 1 ( echo [WARN] test_tensor no compilo ) else ( echo   test_tensor.exe OK )

REM Compilar test_full (integration test)
echo [BONUS 2] Compilando test_full.exe...
cl %CFLAGS% /I.. "..\tests\test_full.c" /Fo"%OUTDIR%\tf_main.obj"
cl %CFLAGS% /I.. "..\tensor\tensor.c" /Fo"%OUTDIR%\tf_tensor.obj"
cl %CFLAGS% /I.. "..\math_ops\nn_ops.c" /Fo"%OUTDIR%\tf_nn_ops.obj"
cl %CFLAGS% /I.. "..\memory\arena.c" /Fo"%OUTDIR%\tf_arena.obj"
cl %CFLAGS% /I.. "..\nn\linear.c" /Fo"%OUTDIR%\tf_linear.obj"
cl %CFLAGS% /I.. "..\autograd\autograd.c" /Fo"%OUTDIR%\tf_autograd.obj"
cl %CFLAGS% /I.. "..\optim\optim.c" /Fo"%OUTDIR%\tf_optim.obj"
cl %CFLAGS% /I.. "..\io\tensor_io.c" /Fo"%OUTDIR%\tf_tensor_io.obj"
link /nologo "%OUTDIR%\tf_main.obj" "%OUTDIR%\tf_tensor.obj" "%OUTDIR%\tf_nn_ops.obj" "%OUTDIR%\tf_arena.obj" "%OUTDIR%\tf_linear.obj" "%OUTDIR%\tf_autograd.obj" "%OUTDIR%\tf_optim.obj" "%OUTDIR%\tf_tensor_io.obj" /OUT:"%OUTDIR%\test_full.exe"
if errorlevel 1 ( echo [WARN] test_full no compilo ) else ( echo   test_full.exe OK )

echo.
echo ======================================================
echo   Build MSVC OK — Runtime 2.0 listo
echo ======================================================
