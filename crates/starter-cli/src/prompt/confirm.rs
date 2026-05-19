//! Yes/no confirmation prompt. Used by destructive subcommands
//! (`admin delete`, `migrate down`).

/// Prompt the user with `message [y/N]`. Returns `true` only on
/// explicit `y`/`Y`.
pub fn confirm(_message: &str) -> std::io::Result<bool> {
    todo!("confirm prompt lands with the first destructive subcommand")
}
