use dioxus::prelude::*;
use crate::backend::utils::set_user_avatar;

struct User {
    id: i32,
    username: String,
    created_at: String,
    avatar: String
}

#[derive(Clone)]
pub struct AuthState {
    user: Signal<Option<User>>
}

impl AuthState {
    pub fn new() -> Self {
        Self { user: Signal::new(None) }
    }

    pub fn login(  &mut self, id: i32, username: String, created_at: String, avatar: Option<Vec<u8>>) {
        let avatar: String = set_user_avatar(avatar);
        self.user.set(Some(User { id, username, created_at, avatar }));
    }
    pub fn logout( &mut self) {
        self.user.set(None);
    }
    pub fn is_logged_in(&self) -> bool {
        self.user.read().is_some()
    }
}
