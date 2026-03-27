use crate::backend::avatar_utils::get_user_avatar_with_default;
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    created_at: String,
    pub avatar: String,
}

#[derive(Clone)]
pub struct AuthState {
    pub user: Signal<Option<User>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            user: Signal::new(None),
        }
    }

    pub fn login(
        &mut self,
        id: i64,
        username: String,
        created_at: String,
        avatar: Option<Vec<u8>>,
    ) {
        let avatar: String = get_user_avatar_with_default(avatar);
        self.user.set(Some(User {
            id,
            username,
            created_at,
            avatar,
        }));
    }
    pub fn logout(&mut self) {
        self.user.set(None);
    }
    pub fn is_logged_in(&self) -> bool {
        self.user.read().is_some()
    }
    pub fn get_avatar(&self) -> String {
        match &*self.user.read() {
            Some(user) => user.avatar.clone(),
            None => {
                // Restituisce l'avatar di default quando non c'è utente
                get_user_avatar_with_default(None)
            }
        }
    }
    pub fn get_username(&self) -> String {
        match &*self.user.read() {
            Some(user) => user.username.clone(),
            None => "".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn set_username(&mut self, username: String) {
        match &mut *self.user.write() {
            Some(user) => user.username = username,
            None => {}
        }
    }

    pub fn get_user(&self) -> Option<User> {
        let user = self.user.read().clone();
        user
    }
    pub fn get_user_id(&self) -> i64 {
        match &*self.user.read() {
            Some(user) => user.id.clone(),
            None => -1,
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}
