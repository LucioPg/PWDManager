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
