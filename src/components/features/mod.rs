pub mod login;
pub mod dashboard;
pub mod settings;
pub mod logout;
pub mod register_user;
mod landing;

pub use login::*;
pub use logout::*;
pub use dashboard::*;
pub use settings::*;
pub use register_user::*;
pub use landing::*;