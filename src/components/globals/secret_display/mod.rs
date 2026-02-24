//! SecretDisplay - Componente per la visualizzazione sicura di dati sensibili
//!
//! Usato per password, location, e altri campi che richiedono protezione visiva.
//! Il valore è nascosto di default e rivelato solo tramite click dell'utente.
//!
//! # Esempio
//! ```rust
//! rsx! {
//!     SecretDisplay {
//!         secret: FormSecret(SecretString::new("my-secret-value".into())),
//!         max_width: "200px".to_string(),
//!     }
//! }
//! ```

mod component;
pub use component::SecretDisplay;
