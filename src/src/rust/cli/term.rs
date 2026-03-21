use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// Cached color state
// ---------------------------------------------------------------------------

static COLOR_CHECKED: AtomicBool = AtomicBool::new(false);
static COLOR_ENABLED: AtomicBool = AtomicBool::new(true);

/// Returns `false` when the `NO_COLOR` environment variable is set.
pub fn is_color_enabled() -> bool {
    if !COLOR_CHECKED.load(Ordering::Relaxed) {
        let enabled = env::var("NO_COLOR").is_err();
        COLOR_ENABLED.store(enabled, Ordering::Relaxed);
        COLOR_CHECKED.store(true, Ordering::Release);
    }
    COLOR_ENABLED.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// ANSI escape-code constants
// ---------------------------------------------------------------------------

// Styles
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const UNDERLINE: &str = "\x1b[4m";

// Standard foreground colours
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";
pub const GRAY: &str = "\x1b[90m";

// Bright foreground colours
pub const BRIGHT_RED: &str = "\x1b[91m";
pub const BRIGHT_GREEN: &str = "\x1b[92m";
pub const BRIGHT_YELLOW: &str = "\x1b[93m";
pub const BRIGHT_BLUE: &str = "\x1b[94m";
pub const BRIGHT_CYAN: &str = "\x1b[96m";

// Background colours
pub const BG_RED: &str = "\x1b[41m";
pub const BG_GREEN: &str = "\x1b[42m";
pub const BG_YELLOW: &str = "\x1b[43m";
pub const BG_BLUE: &str = "\x1b[44m";

// ---------------------------------------------------------------------------
// Windows: enable ANSI / virtual-terminal processing
// ---------------------------------------------------------------------------

/// Enable ANSI escape-sequence processing on the current console.
///
/// On Windows this calls `SetConsoleMode` with
/// `ENABLE_VIRTUAL_TERMINAL_PROCESSING` (0x0004) on the stdout handle.
/// On other platforms this is a no-op.
#[cfg(windows)]
pub fn enable_ansi() {
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> isize;
        fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> i32;
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == -1 {
            return;
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return;
        }
        let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}

#[cfg(not(windows))]
pub fn enable_ansi() {
    // No-op: Unix terminals handle ANSI natively.
}

// ---------------------------------------------------------------------------
// Colour helper – wraps text with escape codes (respects NO_COLOR)
// ---------------------------------------------------------------------------

#[inline]
fn wrap(codes: &str, text: &str) -> String {
    if is_color_enabled() {
        format!("{codes}{text}{RESET}")
    } else {
        text.to_string()
    }
}

// ---------------------------------------------------------------------------
// Public formatting helpers for Step Mode output
// ---------------------------------------------------------------------------

/// Bold bright-blue header for a compiler phase.
pub fn phase_header(text: &str) -> String {
    wrap(&format!("{BOLD}{BRIGHT_BLUE}"), text)
}

/// Bright-green success text.
pub fn ok(text: &str) -> String {
    wrap(BRIGHT_GREEN, text)
}

/// Bright-yellow warning text.
pub fn warn(text: &str) -> String {
    wrap(BRIGHT_YELLOW, text)
}

/// Bold bright-red error text.
pub fn error_text(text: &str) -> String {
    wrap(&format!("{BOLD}{BRIGHT_RED}"), text)
}

/// Cyan informational text.
pub fn info(text: &str) -> String {
    wrap(CYAN, text)
}

/// Gray / dim text.
pub fn dim(text: &str) -> String {
    wrap(&format!("{DIM}{GRAY}"), text)
}

/// Magenta for token display.
pub fn token_fmt(text: &str) -> String {
    wrap(MAGENTA, text)
}

/// Bright cyan for type display.
pub fn type_fmt(text: &str) -> String {
    wrap(BRIGHT_CYAN, text)
}

/// Cyan underlined location string formatted as `file:line:col`.
pub fn loc(file: &str, line: usize, col: usize) -> String {
    let text = format!("{file}:{line}:{col}");
    wrap(&format!("{CYAN}{UNDERLINE}"), &text)
}

/// Prints a coloured separator bar for a compiler phase.
///
/// ```text
/// ── Phase 1: Lexer [C] ──────────────────────────────
/// ```
pub fn phase_bar(phase_num: usize, name: &str, lang: &str) -> String {
    let label = format!("── Phase {phase_num}: {name} [{lang}] ");
    let pad_len = 60usize.saturating_sub(label.len());
    let pad: String = "─".repeat(pad_len);
    let full = format!("{label}{pad}");

    if is_color_enabled() {
        format!("{BOLD}{BRIGHT_BLUE}{full}{RESET}")
    } else {
        full
    }
}

// ---------------------------------------------------------------------------
// Source context: show a source line with a caret under the error location
// ---------------------------------------------------------------------------

/// Renders a source-context snippet with a caret pointing to `(line, col)`.
///
/// ```text
///   12 | int x = foo()
///      |             ^ error: expected ';' after expression
/// ```
///
/// `severity` is used to colour the caret line (`"error"` → red, `"warning"`
/// → yellow, anything else → cyan).
pub fn source_context(source: &str, line: usize, col: usize, msg: &str, severity: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();

    // `line` is 1-based; fall back to an empty line if out of range.
    let src_line = if line >= 1 && line <= lines.len() {
        lines[line - 1]
    } else {
        ""
    };

    let line_num = format!("{line}");
    let gutter_width = line_num.len();

    // The source line itself.
    let source_row = format!("{:>width$} | {}", line_num, src_line, width = gutter_width);

    // Caret row: spaces up to the column, then '^'.
    let col_offset = if col >= 1 { col - 1 } else { 0 };
    let caret_row_plain = format!(
        "{:>width$} | {:>offset$}^ {severity}: {msg}",
        "",
        "",
        width = gutter_width,
        offset = col_offset,
    );

    if !is_color_enabled() {
        return format!("{source_row}\n{caret_row_plain}");
    }

    let color = match severity {
        "error" => format!("{BOLD}{BRIGHT_RED}"),
        "warning" => format!("{BOLD}{BRIGHT_YELLOW}"),
        _ => CYAN.to_string(),
    };

    let caret_row_colored = format!(
        "{:>width$} | {:>offset$}{color}^ {severity}: {msg}{RESET}",
        "",
        "",
        width = gutter_width,
        offset = col_offset,
    );

    format!("{source_row}\n{caret_row_colored}")
}
