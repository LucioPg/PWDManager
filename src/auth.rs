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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Helper: crea un VirtualDom minimo e esegue f nello scope root.
    /// Signal::new() richiede un runtime + uno scope attivo.
    fn with_runtime(f: impl FnOnce()) {
        let dom = VirtualDom::new(|| rsx! {});
        dom.in_scope(dioxus::prelude::ScopeId::ROOT, f);
    }

    #[test]
    fn test_new_auth_state_is_not_logged_in() {
        with_runtime(|| {
            let auth = AuthState::new();
            assert!(!auth.is_logged_in());
            assert_eq!(auth.get_user(), None);
            assert_eq!(auth.get_user_id(), -1);
        });
    }

    #[test]
    fn test_default_trait_creates_valid_state() {
        with_runtime(|| {
            let auth = AuthState::default();
            assert!(!auth.is_logged_in());
            assert_eq!(auth.get_user(), None);
        });
    }

    #[test]
    fn test_login_sets_user() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(42, "alice".into(), "2024-01-01".into(), None);
            assert!(auth.is_logged_in());
            assert_eq!(auth.get_user_id(), 42);
        });
    }

    #[test]
    fn test_login_sets_username() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(1, "bob".into(), "2024-06-15".into(), None);
            assert_eq!(auth.get_username(), "bob");
        });
    }

    #[test]
    fn test_login_without_avatar_returns_default() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(1, "user".into(), "2024-01-01".into(), None);
            assert!(auth.get_avatar().starts_with("data:"));
        });
    }

    #[test]
    fn test_login_with_custom_avatar() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            let avatar_bytes = vec![0x89, 0x50, 0x4E, 0x47]; // fake PNG header
            auth.login(1, "user".into(), "2024-01-01".into(), Some(avatar_bytes));
            assert!(auth.get_avatar().starts_with("data:"));
        });
    }

    #[test]
    fn test_logout_clears_user() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(1, "alice".into(), "2024-01-01".into(), None);
            assert!(auth.is_logged_in());
            auth.logout();
            assert!(!auth.is_logged_in());
            assert_eq!(auth.get_user(), None);
            assert_eq!(auth.get_user_id(), -1);
        });
    }

    #[test]
    fn test_get_username_when_not_logged_in() {
        with_runtime(|| {
            let auth = AuthState::new();
            assert_eq!(auth.get_username(), "");
        });
    }

    #[test]
    fn test_get_avatar_when_not_logged_in() {
        with_runtime(|| {
            let auth = AuthState::new();
            assert!(auth.get_avatar().starts_with("data:"));
        });
    }

    #[test]
    fn test_get_user_id_when_not_logged_in() {
        with_runtime(|| {
            let auth = AuthState::new();
            assert_eq!(auth.get_user_id(), -1);
        });
    }

    #[test]
    fn test_set_username_updates_username() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(1, "old_name".into(), "2024-01-01".into(), None);
            auth.set_username("new_name".into());
            assert_eq!(auth.get_username(), "new_name");
        });
    }

    #[test]
    fn test_set_username_does_nothing_when_not_logged_in() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.set_username("ghost".into());
            assert_eq!(auth.get_username(), "");
        });
    }

    #[test]
    fn test_login_logout_cycle() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            auth.login(1, "alice".into(), "2024-01-01".into(), None);
            assert_eq!(auth.get_user_id(), 1);
            auth.logout();
            assert_eq!(auth.get_user_id(), -1);
            auth.login(99, "bob".into(), "2025-06-01".into(), None);
            assert_eq!(auth.get_user_id(), 99);
            assert_eq!(auth.get_username(), "bob");
        });
    }

    #[test]
    fn test_user_clone_and_equality() {
        let user = User {
            id: 1,
            username: "alice".into(),
            created_at: "2024-01-01".into(),
            avatar: "avatar_data".into(),
        };
        let cloned = user.clone();
        assert_eq!(user, cloned);
    }

    #[test]
    fn test_login_and_logout_trigger_auth_change_callback() {
        with_runtime(|| {
            let mut auth = AuthState::new();
            let calls = std::rc::Rc::new(RefCell::new(Vec::new()));

            set_on_auth_change({
                let calls = calls.clone();
                move |is_logged: bool| calls.borrow_mut().push(is_logged)
            });

            auth.login(1, "alice".into(), "2024-01-01".into(), None);
            assert_eq!(*calls.borrow(), vec![true]);

            auth.logout();
            assert_eq!(*calls.borrow(), vec![true, false]);
        });
    }
}
