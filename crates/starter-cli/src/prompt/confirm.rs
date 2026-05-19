//! Yes/no confirmation prompt. Used by destructive subcommands
//! (`admin delete`, `migrate down`).

use std::io::{BufRead, Write};

/// Prompt the user with `message [y/N]`. Returns `true` only on
/// explicit `y` / `Y` (case-insensitive); anything else — empty
/// line, `n`, garbage — returns `false`. Default is safe (no).
pub fn confirm(message: &str) -> std::io::Result<bool> {
    let mut stdout = std::io::stdout();
    write!(stdout, "{message} [y/N] ")?;
    stdout.flush()?;

    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim(), "y" | "Y"))
}
