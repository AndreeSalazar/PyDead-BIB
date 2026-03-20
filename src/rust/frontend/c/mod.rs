// ============================================================
// ADead-BIB C Frontend — Complete Pipeline
// ============================================================
// C99/C11 → ADead-BIB IR pipeline
//
// C Source → CPreprocessor → CLexer → CParser → CTranslationUnit → CToIR → Program
//
// Modules:
//   c_stdlib       — Built-in C standard library headers (50+ headers)
//   c_preprocessor — #include resolution, #define/#ifdef skipping
//   c_lexer        — Tokenizer: C source → CToken stream
//   c_ast          — C AST types (CExpr, CStmt, CTopLevel, etc.)
//   c_parser       — Recursive descent: CToken → C AST
//   c_to_ir        — C AST → ADead-BIB IR (Program/Function/Stmt/Expr)
//
// Supported libraries for FastOS (all headers built-in):
//   Base:        musl libc, libpthread, libm, libdl
//   Gráficos:    Vulkan, Wayland, libdrm, EGL
//   Fuentes:     FreeType2, HarfBuzz
//   Imágenes:    libpng, libjpeg-turbo, libwebp
//   Compresión:  zlib, lz4, zstd
//   Audio:       Vorbis, Opus, FLAC
//   Red:         libcurl, OpenSSL, sockets, epoll
//   DB:          SQLite3
//   Multimedia:  FFmpeg (avcodec, avformat, avutil, swscale)
//   Hardware:    libinput, xkbcommon, libudev, libusb, libevdev, libpci
//   Compresión+: bzip2, xz/liblzma
//   Imágenes+:   libtiff, libgif
//   Audio+:      libogg, PulseAudio
//   Red+:        libssh2, c-ares
//   Fuentes+:    fontconfig, fribidi
//   DB+:         LevelDB
//   Config:      jsmn (JSON), libconfig
//   XML:         expat
//
// Sin GCC. Sin LLVM. Sin Clang. Solo ADead-BIB. 💀🦈
// ============================================================

pub mod c_ast;
pub mod c_compiler_extensions;
pub mod c_lexer;
pub mod c_parser;
pub mod c_preprocessor;
pub mod c_stdlib;
pub mod c_to_ir;

pub use c_preprocessor::CPreprocessor;
pub use c_to_ir::compile_c_to_program;
