// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

pub mod base_modal;
mod database_reset;
mod export_progress;
mod export_warning;
mod import_progress;
mod import_warning;
mod migration_progress;
mod migration_warning;
mod recovery_key_input;
mod recovery_key_regenerate;
mod recovery_key_setup;
mod stored_all_passwords_deletion;
mod stored_password_deletion;
mod stored_password_show;
mod stored_password_upsert;
pub mod user_deletion;

pub use base_modal::*;
pub use database_reset::*;
pub use recovery_key_input::*;
pub use recovery_key_regenerate::*;
pub use recovery_key_setup::*;
pub use stored_password_deletion::*;
pub use user_deletion::*;

// ide-only serve per avere highlight mentre si lavora su elementi non ancora completati.
// #[cfg(feature = "ide-only")]
pub use export_progress::*;
pub use export_warning::*;
pub use import_progress::*;
pub use import_warning::*;
pub use migration_progress::*;
pub use migration_warning::*;
pub use stored_all_passwords_deletion::*;
pub use stored_password_show::*;
pub use stored_password_upsert::*;
