mod action_icons;
mod alert_icons;
mod base_icon;
mod visibility_icons;

// Re-export del componente base (per estensioni future)
pub use base_icon::SvgIcon;

// Re-export di tutte le icone specifiche
pub use action_icons::{BurgerIcon, ClipboardIcon, DeleteIcon, EditIcon, MagicWandIcon};
pub use alert_icons::{LogoutIcon, WarningIcon};
pub use visibility_icons::{EyeIcon, EyeOffIcon};
