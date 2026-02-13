use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::{
    Decode, Encode, Type,
    encode::IsNull,
    sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};

use sqlx_template::SqlxTemplate;

#[derive(Debug, Clone)]
pub struct DbSecretString(pub SecretString);

impl Type<Sqlite> for DbSecretString {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DbSecretString {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        // Bisogna convertire il secret in una String posseduta
        // affinché sqlx possa gestirne il lifetime correttamente in SQLite
        let val = self.0.expose_secret().to_string();
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretString {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Decodifichiamo il valore come stringa normale delegando a String
        let s = <String as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;

        // 2. Incapsuliamo la stringa nel SecretString (e poi nel tuo wrapper)
        Ok(DbSecretString(SecretString::from(s)))
    }
}

impl From<SecretString> for DbSecretString {
    fn from(secret: SecretString) -> Self {
        Self(secret)
    }
}

impl std::ops::Deref for DbSecretString {
    type Target = SecretString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Per Vec<u8> - il caso più comune per password/binari
pub type SecretSliceU8 = SecretBox<[u8]>;

#[derive(Debug, Clone)]
pub struct DbSecretVec(pub SecretSliceU8);

impl Type<Sqlite> for DbSecretVec {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        // Usiamo BLOB per dati binari, oppure String se serializziamo come hex/base64
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DbSecretVec {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        // Esponiamo il secret e lo convertiamo in Vec<u8> posseduta
        let slice = self.0.expose_secret();
        let val: Vec<u8> = slice.to_vec();
        <Vec<u8> as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretVec {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Decodifichiamo come Vec<u8>
        let vec = <Vec<u8> as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;

        // Convertiamo Vec in SecretBox<[u8]>
        Ok(DbSecretVec(SecretBox::from(vec)))
    }
}
impl From<Vec<u8>> for DbSecretVec {
    fn from(vec: Vec<u8>) -> Self {
        Self(SecretBox::from(vec))
    }
}
impl From<SecretSliceU8> for DbSecretVec {
    fn from(secret: SecretSliceU8) -> Self {
        Self(secret)
    }
}

impl std::ops::Deref for DbSecretVec {
    type Target = SecretSliceU8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(sqlx::FromRow)] // Necessario per mappare i risultati
pub struct UserAuth {
    pub password: DbSecretString,
    pub created_at: String, // o il tipo che usi (es. SystemTime o PrimitiveDateTime)
}

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum PasswordStrength {
    WEAK,
    MEDIUM,
    STRONG,
}

#[derive(sqlx::FromRow, Debug, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct StoredPassword {
    pub id: Option<i64>,            // INTEGER PRIMARY KEY,
    pub user_id: i64,               // INTEGER NOT NULL,
    pub location: String,           // TEXT NOT NULL,
    pub password: DbSecretVec,      // TEXT NOT NULL,
    pub notes: Option<String>,      //,
    pub strength: PasswordStrength, // TEXT NOT NULL CHECK (strength IN ('weak', 'medium', 'strong')),
    pub created_at: Option<String>, // TEXT DEFAULT (datetime('now')),
    pub nonce: Vec<u8>,             // BLOB NOT NULL UNIQUE,
}

impl StoredPassword {
    pub fn new(
        id: Option<i64>,
        user_id: i64,
        location: String,
        password: Vec<u8>,
        notes: Option<String>,
        strength: PasswordStrength,
        created_at: Option<String>,
        nonce: Vec<u8>,
    ) -> Self {
        let password: DbSecretVec = password.into();
        StoredPassword {
            id,
            user_id,
            location,
            password,
            notes,
            strength,
            created_at,
            nonce,
        }
    }
}
