// ============================================================
// fastos_regex.rs — <regex> implementation
// ============================================================
// std::regex, std::smatch, std::regex_search/match/replace
// ============================================================

pub const REGEX_TYPES: &[&str] = &[
    "regex", "wregex", "basic_regex",
    "smatch", "cmatch", "wsmatch", "wcmatch",
    "ssub_match", "csub_match",
    "regex_error",
];

pub const REGEX_FUNCTIONS: &[&str] = &[
    "regex_search", "regex_match", "regex_replace",
];

pub const REGEX_CONSTANTS: &[&str] = &[
    "regex_constants",
];

pub fn is_regex_symbol(name: &str) -> bool {
    REGEX_TYPES.contains(&name) || REGEX_FUNCTIONS.contains(&name) || REGEX_CONSTANTS.contains(&name)
}
