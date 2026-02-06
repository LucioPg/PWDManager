use dioxus::prelude::*;

struct User {
    id: i32,
    username: String,
    created_at: String
}

#[derive(Clone)]
pub struct AuthState {
    user: Signal<Option<User>>
}

impl AuthState {
    pub fn new() -> Self {
        Self { user: Signal::new(None) }
    }

    pub fn login(  &mut self, id: i32, username: String, created_at: String) {
        self.user.set(Some(User { id, username, created_at }));
    }
    pub fn logout( &mut self) {
        self.user.set(None);
    }
    pub fn is_logged_in(&self) -> bool {
        self.user.read().is_some()
    }
}
