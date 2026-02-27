// Base è sempre incluso quando il modulo icons è attivo
mod base;
pub use base::SvgIcon;

// Visibility icons (EyeIcon, EyeOffIcon)
#[cfg(any(feature = "icons-visibility", feature = "icons"))]
mod visibility;
#[cfg(any(feature = "icons-visibility", feature = "icons"))]
pub use visibility::{EyeIcon, EyeOffIcon};

// Action icons (BurgerIcon, ClipboardIcon, DeleteIcon, EditIcon, MagicWandIcon)
#[cfg(any(feature = "icons-action", feature = "icons"))]
mod action;
#[cfg(any(feature = "icons-action", feature = "icons"))]
pub use action::{BurgerIcon, ClipboardIcon, DeleteIcon, EditIcon, MagicWandIcon};

// Alert icons (WarningIcon, LogoutIcon)
#[cfg(any(feature = "icons-alert", feature = "icons"))]
mod alert;
#[cfg(any(feature = "icons-alert", feature = "icons"))]
pub use alert::{WarningIcon, LogoutIcon};
