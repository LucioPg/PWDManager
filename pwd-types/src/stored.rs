//! Tipi database per password salvate.

use crate::{PasswordScore, SecretBox, SecretString};
use secrecy::ExposeSecret;
use sqlx::FromRow;
use sqlx_template::SqlxTemplate;

use crate::{DbSecretString, DbSecretVec};

/// Struct per l'autenticazione utente.
#[derive(FromRow, Debug)]
pub struct UserAuth {
    pub id: i64,
    pub password: DbSecretString,
}

/// Struct per una password salvata nel database.
#[derive(FromRow, Debug, Clone, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct StoredPassword {
    pub id: Option<i64>,
    pub user_id: i64,
    pub location: DbSecretVec,
    pub location_nonce: Vec<u8>,
    pub password: DbSecretVec,
    pub password_nonce: Vec<u8>,
    pub notes: Option<DbSecretVec>,
    pub notes_nonce: Option<Vec<u8>>,
    pub score: PasswordScore,
    pub created_at: Option<String>,
}

impl StoredPassword {
    /// Crea una nuova struct [`StoredPassword`].
    pub fn new(
        id: Option<i64>,
        user_id: i64,
        location: SecretBox<[u8]>,
        location_nonce: Vec<u8>,
        password: SecretBox<[u8]>,
        notes: Option<SecretBox<[u8]>>,
        notes_nonce: Option<Vec<u8>>,
        score: PasswordScore,
        created_at: Option<String>,
        password_nonce: Vec<u8>,
    ) -> Self {
        let location: DbSecretVec = location.into();
        let password: DbSecretVec = password.into();
        let notes: Option<DbSecretVec> = notes.map(|n| n.into());

        StoredPassword {
            id,
            user_id,
            location,
            location_nonce,
            password,
            password_nonce,
            notes,
            notes_nonce,
            score,
            created_at,
        }
    }
}

/// Password non criptata per uso interno.
#[derive(Clone)]
pub struct StoredRawPassword {
    pub id: Option<i64>,
    #[allow(unused)]
    pub user_id: i64,
    pub location: SecretString,
    pub password: SecretString,
    pub notes: Option<SecretString>,
    pub score: Option<PasswordScore>,
    pub created_at: Option<String>,
}

impl std::fmt::Debug for StoredRawPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredRawPassword")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("location", &"***SECRET***")
            .field("password", &"***SECRET***")
            .field("notes", &self.notes.as_ref().map(|_| "***SECRET***"))
            .field("score", &self.score)
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl StoredRawPassword {
    pub fn new() -> Self {
        StoredRawPassword {
            id: None,
            user_id: 0,
            location: SecretString::new("".into()),
            password: "".to_string().into(),
            notes: None,
            score: None,
            created_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_form_fields(
        &self,
    ) -> (
        i64,
        SecretString,
        SecretString,
        Option<SecretString>,
        Option<PasswordScore>,
    ) {
        (
            self.id.unwrap(),
            self.location.clone(),
            self.password.clone(),
            self.notes.clone(),
            self.score.clone(),
        )
    }
}

impl PartialEq for StoredRawPassword {
    fn eq(&self, other: &Self) -> bool {
        match (&self.id, &other.id) {
            (Some(id1), Some(id2)) => {
                id1 == id2
                    && self.location.expose_secret() == other.location.expose_secret()
            }
            (None, None) => true,
            _ => false,
        }
    }
}
