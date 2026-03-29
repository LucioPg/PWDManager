// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! SecretNotesTooltip - Componente per mostrare notes segrete nel tooltip
//!
//! Le notes sono nascoste di default e rivelate solo dopo click dell'utente.
//! Questo garantisce che i dati sensibili non siano mai visibili senza interazione esplicita.

mod component;
pub use component::SecretNotesTooltip;
