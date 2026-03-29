// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! # ToastHub - Re-export from pwd-dioxus
//!
//! This module re-exports the toast system from the pwd-dioxus library.
//! See pwd_dioxus::toast for documentation.

pub use pwd_dioxus::{
    ToastContainer, ToastHubState, schedule_toast_success, show_toast_error, show_toast_success,
    use_toast,
};
