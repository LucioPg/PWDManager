# pwd-strength Library Extraction Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Estrarre la logica di valutazione password in una libreria indipendente `pwd-strength` con approccio TDD.

**Architecture:** Libreria con feature flags per async/tracing, sistema blacklist caricabile da file esterno, dipendenza da `pwd-types` per i tipi condivisi.

**Tech Stack:** Rust, secrecy, tokio, tokio-util, tracing, pwd-types

---

## Prerequisiti

- [x] Step 1 (pwd-types) completato
- [x] Branch `dev-extract-libs-31` attivo (nome corretto)
- [ ] `cargo test --workspace` passa

> **Nota su test flaky (2026-02-26):** Il test `test_cascade_delete_settings_on_user_delete` in `db_settings_tests.rs` è flaky e fallisce intermittente. È indipendente dall'estrazione della libreria pwd-strength, quindi procediamo ignorandolo. Questo test va investigato separatamente.

---

## Struttura Crate Finale

```
pwd-strength/
├── Cargo.toml
└── src/
    ├── lib.rs                 # Public API + re-exports da pwd-types
    ├── blacklist.rs           # Blacklist loader con variabile ambiente
    ├── evaluator.rs           # evaluate_password_strength + cancellation
    └── sections/
        ├── mod.rs
        ├── blacklist.rs       # blacklist_section
        ├── length.rs          # length_section
        ├── variety.rs         # character_variety_section
        └── pattern.rs         # pattern_analysis_section
```

> **Nota:** I test sono inline nei moduli (`#[cfg(test)]`) per semplicità. Non è necessaria una directory `tests/` separata.

---

## Task 1: Setup Directory Structure

**Files:**
- Create: `pwd-strength/Cargo.toml`
- Create: `pwd-strength/src/lib.rs` (vuoto)

**Step 1: Create directory**

```bash
mkdir -p pwd-strength/src/sections
```

**Step 2: Create Cargo.toml**

Create file `pwd-strength/Cargo.toml`:

```toml
[package]
name = "pwd-strength"
version = "0.1.0"
edition = "2024"

[features]
default = ["async"]

# Async support (incluso di default)
async = ["dep:tokio", "dep:tokio-util"]

# Tracing support
tracing = ["dep:tracing"]

[dependencies]
pwd-types = { path = "../pwd-types", features = ["secrecy"] }
thiserror = "2.0"
secrecy = "0.10"

# Async (optional)
tokio = { version = "1", features = ["sync", "time", "rt"], optional = true }
tokio-util = { version = "0.7", features = ["sync"], optional = true }

# Logging (optional)
tracing = { version = "0.1", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros", "sync", "time", "rt"] }
tempfile = "3"
serial_test = "3"  # Per evitare interferenze tra test con OnceLock globale
```

**Step 3: Create placeholder lib.rs**

Create file `pwd-strength/src/lib.rs`:

```rust
//! Password strength evaluation library
//!
//! This library provides password strength evaluation functionality
//! with configurable blacklist support.

// Placeholder - will be filled in later tasks
```

**Step 4: Verify structure**

```bash
ls -la pwd-strength/
```

Expected: Directory structure created

---

## Task 2: Add to Workspace

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Add pwd-strength to workspace members**

Edit `Cargo.toml` line 38:

```toml
[workspace]
members = ["gui_launcher", ".", "custom_errors", "pwd-types", "pwd-strength"]
```

**Step 2: Verify cargo sees the new crate**

```bash
cargo check -p pwd-strength
```

Expected: Compiles without errors (just a warning about unused lib.rs)

---

## Task 3: TDD - Write Blacklist Tests First

**Files:**
- Create: `pwd-strength/src/blacklist.rs`

**Step 1: Write failing tests for blacklist**

Create `pwd-strength/src/blacklist.rs`:

```rust
//! Blacklist management module
//!
//! Handles loading and querying the password blacklist.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;
use thiserror::Error;

static COMMON_PASSWORDS: OnceLock<HashSet<String>> = OnceLock::new();

#[derive(Error, Debug)]
pub enum BlacklistError {
    #[error("Blacklist file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to read blacklist file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Blacklist file is empty")]
    EmptyFile,
}

/// Returns the blacklist file path.
///
/// Priority:
/// 1. Environment variable `PWD_BLACKLIST_PATH`
/// 2. Default path `./assets/10k-most-common.txt`
pub fn get_blacklist_path() -> PathBuf {
    std::env::var("PWD_BLACKLIST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./assets/10k-most-common.txt"))
}

// Placeholder implementations - will be implemented after tests
pub fn init_blacklist() -> Result<usize, BlacklistError> {
    todo!("Implement after writing tests")
}

pub fn get_blacklist() -> Option<&'static HashSet<String>> {
    todo!("Implement after writing tests")
}

pub fn is_blacklisted(password: &str) -> bool {
    todo!("Implement after writing tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use serial_test::serial;  // Evita interferenze tra test con OnceLock globale

    fn reset_blacklist() {
        // SAFETY: This is only for testing purposes
        // We can't reset OnceLock, so tests must use unique passwords
    }

    #[test]
    fn test_get_blacklist_path_default() {
        // Remove env var if set
        std::env::remove_var("PWD_BLACKLIST_PATH");

        let path = get_blacklist_path();
        assert_eq!(path, PathBuf::from("./assets/10k-most-common.txt"));
    }

    #[test]
    fn test_get_blacklist_path_from_env() {
        let custom_path = "/custom/path/blacklist.txt";
        std::env::set_var("PWD_BLACKLIST_PATH", custom_path);

        let path = get_blacklist_path();
        assert_eq!(path, PathBuf::from(custom_path));

        // Cleanup
        std::env::remove_var("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]  // Serial perché modifica OnceLock globale
    fn test_init_blacklist_file_not_found() {
        std::env::set_var("PWD_BLACKLIST_PATH", "/nonexistent/path/blacklist.txt");

        let result = init_blacklist();
        assert!(result.is_err());

        match result {
            Err(BlacklistError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }

        std::env::remove_var("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]  // Serial perché modifica OnceLock globale
    fn test_init_blacklist_empty_file() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "").expect("Failed to write empty content");

        let path = temp_file.path().to_str().unwrap();
        std::env::set_var("PWD_BLACKLIST_PATH", path);

        let result = init_blacklist();
        assert!(matches!(result, Err(BlacklistError::EmptyFile)));

        std::env::remove_var("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]  // Serial perché modifica OnceLock globale
    fn test_init_blacklist_success() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "password123").expect("Failed to write");
        writeln!(temp_file, "qwerty").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        // Use unique env var to avoid collision with other tests
        std::env::set_var("PWD_BLACKLIST_PATH", path);

        let result = init_blacklist();
        assert!(result.is_ok());

        let count = result.unwrap();
        assert_eq!(count, 2);

        std::env::remove_var("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]  // Serial perché usa OnceLock globale
    fn test_is_blacklisted_true() {
        // Create fresh temp file for this test
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "testpassword").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        std::env::set_var("PWD_BLACKLIST_PATH", path);

        // Initialize with our test file
        let _ = init_blacklist();

        assert!(is_blacklisted("testpassword"));
        assert!(is_blacklisted("TESTPASSWORD")); // case insensitive

        std::env::remove_var("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]  // Serial perché usa OnceLock globale
    fn test_is_blacklisted_false() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "common123").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        std::env::set_var("PWD_BLACKLIST_PATH", path);

        let _ = init_blacklist();

        assert!(!is_blacklisted("veryuncommonpassword987"));

        std::env::remove_var("PWD_BLACKLIST_PATH");
    }
}
```

**Step 2: Run tests to verify they fail (todo!() panics)**

```bash
cargo test -p pwd-strength --lib blacklist
```

Expected: Tests fail with "not yet implemented" panic

---

## Task 4: TDD - Implement Blacklist to Pass Tests

**Files:**
- Modify: `pwd-strength/src/blacklist.rs`

**Step 1: Implement init_blacklist**

Replace `init_blacklist` placeholder in `pwd-strength/src/blacklist.rs`:

```rust
/// Initializes the password blacklist from external file.
///
/// # Environment Variable
///
/// Set `PWD_BLACKLIST_PATH` to specify a custom blacklist file location.
/// If not set, defaults to `./assets/10k-most-common.txt`.
///
/// # Errors
///
/// Returns error if:
/// - File does not exist
/// - File cannot be read
/// - File is empty
///
/// # Example
///
/// ```rust,no_run
/// // Custom path via environment
/// std::env::set_var("PWD_BLACKLIST_PATH", "/etc/myapp/blacklist.txt");
/// pwd_strength::init_blacklist()?;
///
/// // Or use default path
/// pwd_strength::init_blacklist()?;
/// ```
pub fn init_blacklist() -> Result<usize, BlacklistError> {
    // Idempotente: se gia inizializzata, ritorna subito
    if COMMON_PASSWORDS.get().is_some() {
        return Ok(COMMON_PASSWORDS.get().map(|s| s.len()).unwrap_or(0));
    }

    let path = get_blacklist_path();

    if !path.exists() {
        return Err(BlacklistError::FileNotFound(path));
    }

    let content = std::fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Err(BlacklistError::EmptyFile);
    }

    let set: HashSet<String> = content
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect();

    let count = set.len();
    let _ = COMMON_PASSWORDS.set(set);

    #[cfg(feature = "tracing")]
    tracing::info!("Blacklist initialized: {} passwords from {:?}", count, path);

    Ok(count)
}
```

**Step 2: Implement get_blacklist**

Replace `get_blacklist` placeholder:

```rust
/// Returns a reference to the loaded blacklist.
///
/// Returns `None` if `init_blacklist()` has not been called.
pub fn get_blacklist() -> Option<&'static HashSet<String>> {
    COMMON_PASSWORDS.get()
}
```

**Step 3: Implement is_blacklisted**

Replace `is_blacklisted` placeholder:

```rust
/// Checks if a password is in the blacklist.
///
/// Returns `true` if password is in the blacklist (case-insensitive).
/// Returns `false` if blacklist is not initialized or password is not found.
pub fn is_blacklisted(password: &str) -> bool {
    COMMON_PASSWORDS
        .get()
        .map(|bl| bl.contains(&password.to_lowercase()))
        .unwrap_or(false)
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p pwd-strength --lib blacklist
```

Expected: All 8 blacklist tests pass

---

## Task 5: TDD - Write Sections Tests

**Files:**
- Create: `pwd-strength/src/sections/mod.rs`
- Create: `pwd-strength/src/sections/blacklist.rs`
- Create: `pwd-strength/src/sections/length.rs`
- Create: `pwd-strength/src/sections/variety.rs`
- Create: `pwd-strength/src/sections/pattern.rs`

**Step 1: Create sections/mod.rs**

```rust
//! Password evaluation sections
//!
//! Each section analyzes a specific aspect of password strength.

mod blacklist;
mod length;
mod pattern;
mod variety;

pub use blacklist::blacklist_section;
pub use length::length_section;
pub use pattern::pattern_analysis_section;
pub use variety::character_variety_section;

/// Result type for section evaluation functions.
/// - `Ok(Some(reason))` - Section failed with reason
/// - `Ok(None)` - Section passed
/// - `Err(())` - Fatal error during evaluation
pub type SectionResult = Result<Option<String>, ()>;
```

**Step 2: Create sections/blacklist.rs with tests**

```rust
//! Blacklist section - checks if password is in common password list.

use crate::blacklist::is_blacklisted;
use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Checks if the password is in the blacklist of common passwords.
///
/// # Returns
/// - `Ok(Some(reason))` if password is blacklisted
/// - `Ok(None)` if password is not in blacklist
pub fn blacklist_section(password: &SecretString) -> SectionResult {
    if is_blacklisted(password.expose_secret()) {
        return Ok(Some(
            "Password is in the top 10,000 most common".to_string(),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup() {
        // Assicura che la blacklist sia inizializzata per i test
        let _ = crate::blacklist::init_blacklist();
    }

    #[test]
    #[serial]  // Serial perché dipende dallo stato globale della blacklist
    fn test_blacklist_section_common_password() {
        setup();
        // password "password" should be in the blacklist file
        let pwd = SecretString::new("password".to_string().into());
        let result = blacklist_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
    }

    #[test]
    #[serial]  // Serial perché dipende dallo stato globale della blacklist
    fn test_blacklist_section_strong_password() {
        setup();
        let pwd = SecretString::new("CorrectHorseBatteryStaple!123".to_string().into());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
```

**Step 3: Create sections/length.rs with tests**

```rust
//! Length section - checks password minimum length.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

const MIN_LENGTH: usize = 8;

/// Checks if the password meets minimum length requirements.
///
/// # Returns
/// - `Ok(Some(reason))` if password is too short
/// - `Ok(None)` if password has sufficient length
pub fn length_section(password: &SecretString) -> SectionResult {
    if password.expose_secret().len() < MIN_LENGTH {
        return Ok(Some(format!(
            "Password must be at least {} characters",
            MIN_LENGTH
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_section_too_short() {
        let pwd = SecretString::new("Short1!".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(
            result,
            Ok(Some("Password must be at least 8 characters".to_string()))
        );
    }

    #[test]
    fn test_length_section_exactly_minimum() {
        let pwd = SecretString::new("12345678".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_length_section_valid() {
        let pwd = SecretString::new("LongEnough123!".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
```

**Step 4: Create sections/variety.rs with tests**

```rust
//! Character variety section - checks for uppercase, lowercase, numbers, special chars.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Checks if the password contains a variety of character types.
///
/// # Returns
/// - `Ok(Some(reason))` if missing required character types
/// - `Ok(None)` if all character types are present
pub fn character_variety_section(password: &SecretString) -> SectionResult {
    let pwd = password.expose_secret();
    let has_upper = pwd.chars().any(|c| c.is_uppercase());
    let has_lower = pwd.chars().any(|c| c.is_lowercase());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
    let has_special = pwd.chars().any(|c| !c.is_alphanumeric());

    let missing: Vec<_> = vec![
        if !has_upper { Some("uppercase") } else { None },
        if !has_lower { Some("lowercase") } else { None },
        if !has_digit { Some("numbers") } else { None },
        if !has_special { Some("special characters") } else { None },
    ]
    .into_iter()
    .flatten()
    .collect();

    if !missing.is_empty() {
        return Ok(Some(format!("Missing: {}", missing.join(", "))));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variety_section_missing_uppercase() {
        let pwd = SecretString::new("lowercase123!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("uppercase"));
        }
    }

    #[test]
    fn test_variety_section_missing_lowercase() {
        let pwd = SecretString::new("UPPERCASE123!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("lowercase"));
        }
    }

    #[test]
    fn test_variety_section_missing_numbers() {
        let pwd = SecretString::new("NoNumbers!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("numbers"));
        }
    }

    #[test]
    fn test_variety_section_missing_special() {
        let pwd = SecretString::new("NoSpecial123".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("special"));
        }
    }

    #[test]
    fn test_variety_section_all_categories() {
        let pwd = SecretString::new("HasAll123!@#".to_string().into());
        let result = character_variety_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
```

**Step 5: Create sections/pattern.rs with tests**

```rust
//! Pattern analysis section - detects repetitive and sequential patterns.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Analyzes password for repetitive and sequential patterns.
///
/// # Returns
/// - `Ok(Some(reason))` if problematic patterns found
/// - `Ok(None)` if no problematic patterns
pub fn pattern_analysis_section(password: &SecretString) -> SectionResult {
    let chars: Vec<char> = password.expose_secret().chars().collect();
    if chars.len() < 3 {
        return Ok(None);
    }

    // Check repeated chars (e.g., "aaa")
    let mut repeated_count = 1;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            repeated_count += 1;
            if repeated_count >= 3 {
                return Ok(Some("Password contains repetitive patterns".to_string()));
            }
        } else {
            repeated_count = 1;
        }
    }

    // Check for longer sequences (4+ consecutive characters)
    for window_size in [4, 5] {
        if chars.len() < window_size {
            continue;
        }

        for i in window_size..=chars.len() {
            let window = &chars[i - window_size..i];

            // Check if all characters in window are sequential
            let is_sequential = window.windows(2).all(|w| {
                let prev = w[0] as i32;
                let curr = w[1] as i32;
                curr == prev + 1 || curr == prev - 1
            });

            if is_sequential {
                return Ok(Some("Password contains sequential patterns".to_string()));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_section_repetitive_chars() {
        let pwd = SecretString::new("aaaaBBBB1111".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("repetitive"));
        }
    }

    #[test]
    fn test_pattern_section_sequential_numbers() {
        let pwd = SecretString::new("test1234abcd".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("sequential"));
        }
    }

    #[test]
    fn test_pattern_section_sequential_letters() {
        let pwd = SecretString::new("abcdTest123".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("sequential"));
        }
    }

    #[test]
    fn test_pattern_section_strong_password() {
        let pwd = SecretString::new("RandomPass123!@#Word".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_pattern_section_too_short() {
        let pwd = SecretString::new("ab".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
```

**Step 6: Run tests to verify they pass**

```bash
cargo test -p pwd-strength --lib sections
```

Expected: All section tests pass

---

## Task 6: TDD - Write Evaluator Tests

**Files:**
- Create: `pwd-strength/src/evaluator.rs`

**Step 1: Create evaluator.rs with tests first**

```rust
//! Password strength evaluator - main evaluation logic.

use pwd_types::{PasswordEvaluation, PasswordScore};
use secrecy::{ExposeSecret, SecretString};

#[cfg(feature = "async")]
use tokio::sync::mpsc;

#[cfg(feature = "async")]
use tokio_util::sync::CancellationToken;

use crate::sections::{
    blacklist_section, character_variety_section, length_section, pattern_analysis_section,
};

/// Evaluates password strength and returns a detailed evaluation.
///
/// # Arguments
/// * `password` - The password to evaluate
/// * `token` - Optional cancellation token (async feature)
///
/// # Returns
/// A `PasswordEvaluation` containing score and reasons.
pub fn evaluate_password_strength(
    password: &SecretString,
    #[cfg(feature = "async")] token: Option<CancellationToken>,
) -> PasswordEvaluation {
    // Placeholder - implement after tests
    todo!("Implement after writing tests")
}

/// Async version that sends evaluation result via channel.
#[cfg(feature = "async")]
pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>,
) {
    use std::time::Duration;

    #[cfg(feature = "tracing")]
    tracing::info!("evaluation is about to start...");

    tokio::time::sleep(Duration::from_millis(300)).await;
    let evaluation = evaluate_password_strength(password, Some(token));

    if let Err(e) = tx.send(evaluation).await {
        #[cfg(feature = "tracing")]
        tracing::error!("Failed to send password evaluation result: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwd_types::PasswordStrength;

    fn setup() {
        // Ensure blacklist is initialized for tests
        let _ = crate::blacklist::init_blacklist();
    }

    #[test]
    fn test_evaluate_weak_short_password() {
        setup();
        let pwd = SecretString::new("abc".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(evaluation.score.is_some());
        assert!(evaluation.score.unwrap() < 50);
        assert!(!evaluation.reasons.is_empty());
    }

    #[test]
    fn test_evaluate_medium_password() {
        setup();
        let pwd = SecretString::new("MyPass123!".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::MEDIUM);
        let score = evaluation.score.unwrap();
        assert!(score >= 50 && score < 70, "Expected MEDIUM score (50-69), got {}", score);
    }

    #[test]
    fn test_evaluate_strong_password() {
        setup();
        let pwd = SecretString::new("VeryStrongPassword123!@#".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert!(matches!(
            evaluation.strength(),
            PasswordStrength::STRONG | PasswordStrength::EPIC | PasswordStrength::GOD
        ));
        assert!(evaluation.score.unwrap() >= 70);
    }

    #[test]
    fn test_evaluate_blacklisted_password() {
        setup();
        let pwd = SecretString::new("password".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        let has_blacklist_reason = evaluation.reasons.iter()
            .any(|r| r.contains("10,000") || r.contains("common"));
        assert!(has_blacklist_reason);
    }

    #[test]
    fn test_evaluate_empty_password() {
        setup();
        let pwd = SecretString::new("".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(!evaluation.reasons.is_empty());
    }

    #[test]
    fn test_evaluate_score_boundaries() {
        setup();
        let test_passwords = vec![
            "",
            "a",
            "password",
            "MyPass123!",
            "VeryStrongPassword123!@#",
        ];

        for pwd_str in test_passwords {
            let pwd = SecretString::new(pwd_str.to_string().into());

            #[cfg(feature = "async")]
            let evaluation = evaluate_password_strength(&pwd, None);

            #[cfg(not(feature = "async"))]
            let evaluation = evaluate_password_strength(&pwd);

            if let Some(score) = evaluation.score {
                assert!(
                    score >= 0 && score <= 100,
                    "Score {} out of bounds for password '{}'",
                    score,
                    pwd_str
                );
            }
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;

    fn setup() {
        let _ = crate::blacklist::init_blacklist();
    }

    #[tokio::test]
    async fn test_evaluate_with_cancellation() {
        setup();
        let token = CancellationToken::new();
        token.cancel();

        let pwd = SecretString::new("SomePassword123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        assert_eq!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(evaluation.score.is_none());
        assert!(!evaluation.reasons.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_without_cancellation() {
        setup();
        let token = CancellationToken::new();

        let pwd = SecretString::new("TestPass123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        assert_ne!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(evaluation.score.is_some());
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_tx() {
        setup();
        let (tx, mut rx) = mpsc::channel(1);
        let token = CancellationToken::new();

        let pwd = SecretString::new("TestPass123!".to_string().into());

        evaluate_password_strength_tx(&pwd, token, tx).await;

        let evaluation = rx.recv().await.expect("Should receive evaluation");
        assert!(evaluation.score.is_some());
    }
}
```

**Step 2: Run tests to verify they fail (todo!() panics)**

```bash
cargo test -p pwd-strength --lib evaluator
```

Expected: Tests fail with "not yet implemented"

---

## Task 7: TDD - Implement Evaluator to Pass Tests

**Files:**
- Modify: `pwd-strength/src/evaluator.rs`

**Step 1: Implement evaluate_password_strength**

Replace the placeholder `evaluate_password_strength` with:

```rust
/// Evaluates password strength and returns a detailed evaluation.
///
/// # Arguments
/// * `password` - The password to evaluate
/// * `token` - Optional cancellation token (async feature only)
///
/// # Returns
/// A `PasswordEvaluation` containing score and reasons.
pub fn evaluate_password_strength(
    password: &SecretString,
    #[cfg(feature = "async")] token: Option<CancellationToken>,
) -> PasswordEvaluation {
    let mut reasons = Vec::new();
    let mut is_cancelled = false;
    let mut score: Option<i64> = None;

    let pwd = password.expose_secret();
    let pwd_len = pwd.len();

    // Orchestrator: execute sections in sequence
    let sections: Vec<(&str, fn(&SecretString) -> Result<Option<String>, ()>)> = vec![
        ("blacklist", blacklist_section),
        ("length", length_section),
        ("variety", character_variety_section),
        ("pattern", pattern_analysis_section),
    ];

    for (section_name, section_fn) in sections {
        // Check cancellation before each section (async only)
        #[cfg(feature = "async")]
        {
            if let Some(ref t) = token {
                if t.is_cancelled() {
                    reasons.push("Evaluation cancelled".to_string());
                    is_cancelled = true;
                    break;
                }
            }
        }

        match section_fn(password) {
            Ok(Some(reason)) => {
                reasons.push(reason);
            }
            Ok(None) => {
                // Section passed, continue
            }
            Err(()) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Fatal error in password evaluation section: {}", section_name);
                reasons.push("Error".to_string());
                score = None;
                break;
            }
        }
    }

    // Calculate strength and final score
    if !is_cancelled {
        // Length bonus: up to 20 points (0.5 per character, max 20)
        let bonus = (pwd_len as f64 * 0.5).min(20.0) as i64;
        let score_ref = score.get_or_insert(0);
        *score_ref += bonus;

        // Character variety: up to 60 points (15 per type)
        let has_upper = pwd.chars().any(|c| c.is_uppercase());
        let has_lower = pwd.chars().any(|c| c.is_lowercase());
        let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
        let has_special = pwd.chars().any(|c| !c.is_alphanumeric());
        let variety_count = [has_upper, has_lower, has_digit, has_special]
            .iter()
            .filter(|&&b| b)
            .count();
        let score_ref = score.get_or_insert(0);
        *score_ref += (variety_count * 15) as i64;

        // Extra length bonus: +5 if > 12, +10 if > 16
        let score_ref = score.get_or_insert(0);
        if pwd_len > 16 {
            *score_ref += 10;
        } else if pwd_len > 12 {
            *score_ref += 5;
        }

        // Multiple special chars bonus: +5 if 2+ special chars
        let special_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();
        if special_count >= 2 {
            let score_ref = score.get_or_insert(0);
            *score_ref += 5;
        }

        // Entropy bonus: based on unique chars
        let unique_chars: std::collections::HashSet<char> = pwd.chars().collect();
        let unique_count = unique_chars.len();
        let score_ref = score.get_or_insert(0);
        if unique_count >= 16 {
            *score_ref += 10;
        } else if unique_count >= 12 {
            *score_ref += 5;
        }

        // Penalties for reasons (each reason subtracts points)
        let score_ref = score.get_or_insert(0);
        *score_ref -= (reasons.len() as i64) * 10;
    }

    PasswordEvaluation {
        score: score.map(|s| PasswordScore::new(s)),
        reasons,
    }
}
```

**Step 2: Run tests to verify they pass**

```bash
cargo test -p pwd-strength --lib evaluator
```

Expected: All evaluator tests pass

---

## Task 8: Wire Up lib.rs Public API

**Files:**
- Modify: `pwd-strength/src/lib.rs`

**Step 1: Create complete lib.rs**

```rust
//! Password strength evaluation library
//!
//! This library provides password strength evaluation functionality
//! with configurable blacklist support.
//!
//! # Features
//!
//! - `async` (default): Enables async evaluation with cancellation support
//! - `tracing`: Enables logging via tracing crate
//!
//! # Environment Variables
//!
//! - `PWD_BLACKLIST_PATH`: Custom path to blacklist file
//!   (default: `./assets/10k-most-common.txt`)
//!
//! # Example
//!
//! ```rust,no_run
//! use pwd_strength::{init_blacklist, evaluate_password_strength};
//! use secrecy::SecretString;
//!
//! // Initialize blacklist (call once at startup)
//! init_blacklist().expect("Failed to load blacklist");
//!
//! // Evaluate a password
//! let password = SecretString::new("MyP@ssw0rd!".to_string().into());
//!
//! #[cfg(feature = "async")]
//! let evaluation = evaluate_password_strength(&password, None);
//!
//! #[cfg(not(feature = "async"))]
//! let evaluation = evaluate_password_strength(&password);
//!
//! println!("Score: {:?}", evaluation.score);
//! println!("Strength: {:?}", evaluation.strength());
//! ```

// Re-export types from pwd-types for convenience
pub use pwd_types::{PasswordEvaluation, PasswordScore, PasswordStrength};

// Internal modules
mod blacklist;
mod evaluator;
mod sections;

// Public API
pub use blacklist::{init_blacklist, get_blacklist, is_blacklisted, BlacklistError};
pub use evaluator::evaluate_password_strength;

#[cfg(feature = "async")]
pub use evaluator::evaluate_password_strength_tx;
```

**Step 2: Verify crate compiles**

```bash
cargo check -p pwd-strength --all-features
```

Expected: No errors

---

## Task 9: Run All Tests

**Files:**
- Test: `pwd-strength/` (all tests)

**Step 1: Run full test suite**

```bash
cargo test -p pwd-strength --all-features
```

Expected: All tests pass

**Step 2: Run with default features only**

```bash
cargo test -p pwd-strength
```

Expected: All tests pass (async tests should be included as async is default)

---

## Task 10: Update PWDManager Dependencies

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Add pwd-strength dependency to PWDManager**

Add to `[dependencies]` section in root `Cargo.toml`:

```toml
pwd-strength = { path = "pwd-strength", features = ["async", "tracing"] }
```

The line should be added after `pwd-types` line (around line 10).

**Step 2: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: No errors (may have warnings about unused imports)

---

## Task 11: Update PWDManager Code to Use Library

**Files:**
- Modify: `src/backend/mod.rs`
- Modify: `src/backend/strength_utils.rs`
- Delete: `src/backend/strength_utils.rs` (eventualmente)

**Step 1: Update mod.rs to re-export from library**

Edit `src/backend/mod.rs` to add re-export:

```rust
// Re-export pwd-strength for backward compatibility
pub use pwd_strength::{
    init_blacklist, is_blacklisted, evaluate_password_strength,
    evaluate_password_strength_tx,
};
```

**Step 2: Update imports in components**

Find all files using `strength_utils` and update imports:

```bash
grep -r "strength_utils" src/
```

For each file found, update the import from:
```rust
use crate::backend::strength_utils::...;
```

To:
```rust
use crate::backend::...;  // via re-export
// OR
use pwd_strength::...;
```

**Step 3: Remove old strength_utils.rs**

After verifying all imports work:

```bash
# Su Unix/Git Bash:
rm src/backend/strength_utils.rs

# Su Windows CMD (alternativa):
# del src\backend\strength_utils.rs
```

And remove from `mod.rs`:
```rust
// Remove this line:
// pub mod strength_utils;
```

**Step 4: Verify compilation**

```bash
cargo check --workspace
```

Expected: No errors

---

## Task 12: Run Full Test Suite

**Files:**
- Test: All workspace tests

**Step 1: Run workspace tests**

```bash
cargo test --workspace
```

Expected: All tests pass

**Step 2: Run with all features**

```bash
cargo test --workspace --all-features
```

Expected: All tests pass

---

## Task 13: Commit Changes

**Files:**
- All modified files

**Step 1: Stage all changes**

```bash
git add pwd-strength/
git add Cargo.toml
git add src/backend/mod.rs
git add src/backend/strength_utils.rs  # if deleted
```

**Step 2: Create commit**

```bash
git commit -m "$(cat <<'EOF'
feat: extract pwd-strength library

- Create pwd-strength crate with TDD approach
- Implement blacklist loader with PWD_BLACKLIST_PATH env var
- Extract password evaluation sections (blacklist, length, variety, pattern)
- Add async support with cancellation token
- Add tracing feature for logging
- Update PWDManager to use new library via re-exports

Breaking changes: None (backward compatible via re-exports)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

**Step 3: Verify commit**

```bash
git log -1 --oneline
```

Expected: Commit created successfully

---

## Task 14: Update Orchestrator Document

**Files:**
- Modify: `docs/plans/2026-02-26-library-extraction-orchestrator.md`

**Step 1: Update Step 2 status**

Change line 34 from:
```markdown
| 2    | `pwd-strength` | ⏳ NON INIZIATO | `docs/plans/2026-02-26-extract-pwd-strength.md` | -          |
```

To:
```markdown
| 2    | `pwd-strength` | ✅ COMPLETATO  | `docs/plans/2026-02-26-extract-pwd-strength.md` | 2026-02-26 |
```

**Step 2: Update Lezioni Apprese section**

Add new entry in "Dopo Step 2 (pwd-strength)" section:

```markdown
### Dopo Step 2 (pwd-strength)

| Aspetto | Riscontrato | Azione |
|---------|-------------|--------|
| OnceLock in tests | Non è possibile resettare OnceLock tra test | Ogni test usa password uniche o file temp separati |
| Feature flags async | Test async richiedono feature async attiva | Async è default, ma test espliciti usano `#[cfg(feature = "async")]` |
| Blacklist path | Default path relativo alla working directory | Documentato chiaramente, test usano tempfile |
```

**Step 3: Update Changelog**

Add entry:
```markdown
| 2026-02-26 | 1.5 | Completato Step 2 (pwd-strength) |
```

---

## Task 15: Update Reference Document

**Files:**
- Modify: `docs/library-extraction-analysis.md`

**Step 1: Update Step 2 checklist**

In "Step 2: pwd-strength - Dettaglio Implementazione" section, mark all items as completed:

```markdown
### Checklist Implementazione

- [x] Creare directory `pwd-strength/`
- [x] Configurare `Cargo.toml` con features
- [x] Implementare `blacklist_loader.rs` con variabile d'ambiente
- [x] Estrarre sezioni in `sections/` directory
- [x] Estrarre `evaluate_password_strength` in `evaluator.rs`
- [x] Aggiornare `PWDManager/Cargo.toml` con dipendenza
- [x] Copiare file `10k-most-common.txt` in `PWDManager/assets/`
- [x] Aggiornare `strength_utils.rs` per usare la libreria
- [x] Rimuovere vecchio `strength_utils.rs` (o fare da re-export)
- [x] Eseguire `cargo test`
- [x] Commit: `feat: extract pwd-strength library`

**Completato:** 2026-02-26
```

**Step 2: Add problems encountered section**

```markdown
### Modifiche rispetto al piano originale

| Aspetto | Piano Originale | Implementazione Effettiva |
|---------|-----------------|---------------------------|
| Blacklist embedded | include_str! con file embedded | Caricamento da file esterno con variabile ambiente |
| Nome modulo blacklist | `blacklist_loader.rs` | `blacklist.rs` (più semplice) |

### Problemi incontrati e soluzioni

1. **OnceLock immutabile**: Non è possibile resettare `OnceLock` tra test. Soluzione: ogni test usa password uniche o file temporanei separati.

2. **Test async condizionali**: I test async devono essere annotati con `#[cfg(feature = "async")]` per evitare errori di compilazione quando la feature è disabilitata.
```

---

## Verification Checklist

Before marking complete:

- [ ] `cargo test -p pwd-strength --all-features` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo check --workspace` completes without errors
- [ ] All imports updated in PWDManager
- [ ] Old `strength_utils.rs` removed
- [ ] Commit created with descriptive message
- [ ] Orchestrator document updated
- [ ] Reference document updated

---

## Next Steps

After completing this plan:
1. Verify Step 2 checkpoint with human review
2. Proceed to Step 3 (pwd-crypto extraction) or Step F (Finalization)
