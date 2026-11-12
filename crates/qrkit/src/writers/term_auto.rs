//! Terminal capability auto-detection for `--format auto`.
//!
//! Env-var heuristics only (no DA1 round-trips — those need a raw tty and
//! risk hangs in pipes). Falls back to plain ANSI half-blocks, which every
//! terminal can show.

/// The detected inline-image strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermImage {
    /// Kitty graphics protocol (Kitty, Ghostty, WezTerm).
    Kitty,
    /// iTerm2 OSC 1337 inline images (iTerm2, Konsole, mintty).
    Iterm,
    /// Plain ANSI half-blocks — universal fallback.
    AnsiBlocks,
}

/// Detect the best supported strategy from the environment.
pub fn detect() -> TermImage {
    detect_with(&|k| std::env::var(k).ok())
}

/// Testable core: `env` abstracts the environment lookup.
pub fn detect_with(env: &dyn Fn(&str) -> Option<String>) -> TermImage {
    // Kitty sets KITTY_WINDOW_ID; Ghostty advertises in TERM; WezTerm sets
    // WEZTERM_EXECUTABLE (and speaks the kitty protocol).
    if env("KITTY_WINDOW_ID").is_some() || env("WEZTERM_EXECUTABLE").is_some() {
        return TermImage::Kitty;
    }
    if let Some(term) = env("TERM") {
        if term.contains("kitty") || term.contains("ghostty") {
            return TermImage::Kitty;
        }
    }
    if let Some(tp) = env("TERM_PROGRAM") {
        if tp == "iTerm.app" || tp == "vscode" {
            return TermImage::Iterm;
        }
    }
    if env("LC_TERMINAL").is_some_and(|v| v.starts_with("iTerm")) {
        return TermImage::Iterm;
    }
    TermImage::AnsiBlocks
}
