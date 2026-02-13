//! Modulo per la gestione dell'autenticazione e delle password nel database.
//!
//! Fornisce wrapper per i tipi `secrecy` che li rendono compatibili con SQLx,
//! oltre a struct per l'autenticazione utente e per le password salvate.

use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::{
    Decode, Encode, Type,
    encode::IsNull,
    sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};

use sqlx_template::SqlxTemplate;

/// Wrapper per [`SecretString`] che lo rende compatibile con SQLx/SQLite.
///
/// SQLx richiede che i tipi implementino trait specifici per essere codificati/decodificati
/// nel database. Questo wrapper implementa tali trait espondo temporaneamente il segreto
/// interno quando necessario per la codifica SQLite.
///
/// # Esempi
///
/// ```rust,no_run
/// use user_auth_helper::DbSecretString;
/// use secrecy::SecretString;
///
/// let secret = SecretString::new("mia_password".into());
/// let db_secret = DbSecretString(secret);
/// // Ora db_secret può essere usato con SQLx
/// ```
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

/// Implementazione di [`From<SecretString>`] per [`DbSecretString`].
///
/// Permette di convertire facilmente una `SecretString` in `DbSecretString`
/// usabile con SQLx.
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

/// Wrapper per [`SecretBox<[u8]>`] che lo rende compatibile con SQLx/SQLite.
///
/// Utilizzato per salvare dati binari criptati (come le password) nel database SQLite.
/// Il nonce usato per la criptazione AES-GCM è anch'esso un `Vec<u8>`.
///
/// # Esempi
///
/// ```rust,no_run
/// use user_auth_helper::DbSecretVec;
/// use secrecy::SecretBox;
///
/// let data: Vec<u8> = vec
/// 0x01, 0x02, 0x03
/// ];
/// let db_secret = DbSecretVec(SecretBox::from(data));
/// // Ora db_secret può essere usato con SQLx
/// ```
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
/// Struct per l'autenticazione utente contenente password e data di creazione.
///
/// Utilizzata per recuperare la password hash e la data di creazione di un utente,
/// necessarie per la derivazione della chiave di criptazione AES.
///
/// # Campi
///
/// * `password` - Password hash (Argon2) dell'utente
/// * `created_at` - Data di creazione dell'utente in formato ISO 8601
pub struct UserAuth {
    pub password: DbSecretString,
    pub created_at: String, // o il tipo che usi (es. SystemTime o PrimitiveDateTime)
}

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
/// Enum che rappresenta la forze della password.
///
/// Viene salvata nel database come testo ('weak', 'medium', 'strong') e
/// controllata da un constraint CHECK.
///
/// # Varianti
///
/// * `WEAK` - Password debole (< 8 caratteri)
/// * `MEDIUM` - Password media (8-15 caratteri)
/// * `STRONG` - Password forte (16+ caratteri)
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
/// Struct per una password salvata nel database.
///
/// Utilizza [`sqlx_template`](SqlxTemplate) per generare automaticamente le query SQL
/// di INSERT/UPDATE/SELECT tramite il metodo `upsert_by_id`.
///
/// # Campi
///
/// * `id` - ID opzionale (None per INSERT, Some per UPDATE)
/// * `user_id` - ID dell'utente proprietario della password
/// * `location` - Luogo/nome dove è salvata la password (es. "Google", "Netflix")
/// * `password` - Password criptata con AES-256-GCM (salvata come BLOB)
/// * `notes` - Note opzionali sulla password
/// * `strength` - Forze della password (WEAK/MEDIUM/STRONG)
/// * `created_at` - Data di creazione opzionale
/// * `nonce` - Nonce usato per la criptazione AES (12 byte, deve essere UNIQUE)
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
    /// Crea una nuova struct [`StoredPassword`] convertendo la password in [`DbSecretVec`].
    ///
    /// # Parametri
    ///
    /// * `id` - ID opzionale (None per nuove password)
    /// * `user_id` - ID dell'utente proprietario
    /// * `location` - Luogo dove è salvata la password
    /// * `password` - Password criptata come bytes
    /// * `notes` - Note opzionali
    /// * `strength` - Forze della password
    /// * `created_at` - Data di creazione opzionale
    /// * `nonce` - Nonce usato per la criptazione AES
    ///
    /// # Valore Restituito
    ///
    /// Return `StoredPassword` con la password avvolta in `DbSecretVec`
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
