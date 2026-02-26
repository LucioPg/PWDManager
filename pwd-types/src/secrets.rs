//! Wrapper secrecy per SQLx/SQLite.
//!
//! Questi wrapper rendono `SecretString` e `SecretBox<[u8]>` compatibili con SQLx.

use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::{sqlite::Sqlite, Type};

/// Type alias per `SecretBox<[u8]>`.
pub type SecretSliceU8 = SecretBox<[u8]>;

/// Wrapper per [`SecretString`] compatibile con SQLx/SQLite.
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
        let val = self.0.expose_secret().to_string();
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretString {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <String as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
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

/// Wrapper per [`SecretBox<[u8]>`] compatibile con SQLx/SQLite.
#[derive(Debug, Clone)]
pub struct DbSecretVec(pub SecretSliceU8);

impl Type<Sqlite> for DbSecretVec {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DbSecretVec {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let slice = self.0.expose_secret();
        let val: Vec<u8> = slice.to_vec();
        <Vec<u8> as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretVec {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let vec = <Vec<u8> as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
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
