// ============================================================
// fastos_filesystem.rs — <filesystem> implementation
// ============================================================
// std::filesystem::path, directory_iterator, etc.
// ============================================================

pub const FS_TYPES: &[&str] = &[
    "path", "directory_iterator", "recursive_directory_iterator",
    "directory_entry", "file_status", "space_info",
    "file_type", "perms", "filesystem_error",
];

pub const FS_FUNCTIONS: &[&str] = &[
    "exists", "is_directory", "is_regular_file", "is_symlink",
    "is_empty", "is_block_file", "is_character_file", "is_fifo", "is_socket", "is_other",
    "create_directory", "create_directories",
    "copy", "copy_file", "copy_symlink",
    "rename", "remove", "remove_all",
    "file_size", "hard_link_count", "last_write_time",
    "current_path", "temp_directory_path", "absolute", "canonical", "relative",
    "status", "symlink_status", "equivalent",
    "resize_file", "space", "permissions",
    "create_hard_link", "create_symlink", "create_directory_symlink",
    "read_symlink",
];

pub const FS_METHODS: &[&str] = &[
    "filename", "extension", "parent_path", "stem", "root_name",
    "root_directory", "root_path", "relative_path",
    "has_filename", "has_extension", "has_parent_path", "has_stem",
    "has_root_name", "has_root_directory", "has_root_path", "has_relative_path",
    "is_absolute", "is_relative",
    "string", "wstring", "u8string", "generic_string",
    "replace_filename", "replace_extension", "remove_filename",
    "make_preferred", "lexically_normal", "lexically_relative", "lexically_proximate",
];

pub fn is_filesystem_symbol(name: &str) -> bool {
    FS_TYPES.contains(&name) || FS_FUNCTIONS.contains(&name) || FS_METHODS.contains(&name)
}
