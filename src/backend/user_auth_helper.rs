use secrecy::{ExposeSecret, SecretString};
use sqlx::{Encode, Sqlite, Type};

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

#[derive(sqlx::FromRow)] // Necessario per mappare i risultati
pub struct UserAuth {
    pub password: DbSecretString,
    pub created_at: String, // o il tipo che usi (es. SystemTime o PrimitiveDateTime)
}
