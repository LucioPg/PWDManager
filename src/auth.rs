// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::backend::avatar_utils::get_user_avatar_with_default;
use dioxus::prelude::*;
use std::cell::RefCell;

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

type AuthChangeCallback = Box<dyn Fn(bool)>;

thread_local! {
    static ON_AUTH_CHANGE: RefCell<Option<AuthChangeCallback>> = RefCell::new(None);
}

/// Registers a callback that fires on every login/logout.
/// Must be called once during app initialization.
pub fn set_on_auth_change(f: impl Fn(bool) + 'static) {
    ON_AUTH_CHANGE.with(|c| { *c.borrow_mut() = Some(Box::new(f)); });
}

fn notify_auth_change(is_logged: bool) {
    ON_AUTH_CHANGE.with(|c| {
        if let Some(cb) = c.borrow().as_ref() {
            cb(is_logged);
        }
    });
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
        notify_auth_change(true);
    }
    pub fn logout(&mut self) {
        self.user.set(None);
        notify_auth_change(false);
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
        if let Some(user) = &mut *self.user.write() {
            user.username = username
        }
    }

    pub fn get_user(&self) -> Option<User> {
        self.user.read().clone()
    }
    pub fn get_user_id(&self) -> i64 {
        match &*self.user.read() {
            Some(user) => user.id,
            None => -1,
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}
