//! PasswordDisplay - Componente per la visualizzazione sicura delle password
//!
//! Questo modulo fornisce un componente che mostra le password mascherate (•••••)
//! con la possibilità di rivelarle in chiaro tramite un toggle.
//!
//! # Sicurezza
//! - La password è sempre passata come `FormSecret` (wrapper di `SecretString`)
//! - L'input è `readonly` per prevenire modifiche accidentali
//! - Il tooltip con la password visibile appare solo quando sbloccata
//!
//! # Esempio
//! ```rust
//! use crate::components::globals::password_display::PasswordDisplay;
//! use crate::components::globals::form_field::FormSecret;
//! use secrecy::SecretString;
//!
//! rsx! {
//!     PasswordDisplay {
//!         password: FormSecret(SecretString::new("my-password".into())),
//!         max_width: "200px".to_string(),
//!     }
//! }
//! ```

mod component;

pub use component::PasswordDisplay;
