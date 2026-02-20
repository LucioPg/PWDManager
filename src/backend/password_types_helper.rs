//! Modulo per la gestione dell'autenticazione e delle password nel database.
//!
//! Fornisce wrapper per i tipi `secrecy` che li rendono compatibili con SQLx,
//! oltre a struct per l'autenticazione utente e per le password salvate.

use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::{Type, sqlite::Sqlite};
use std::collections::HashSet;
use std::fmt;

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
/// let data: Vec<u8> = vec [
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

#[derive(sqlx::FromRow, Debug)] // Necessario per mappare i risultati
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
    pub id: i64,
    pub password: DbSecretString,
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
/// * `score` - Forze della password (WEAK/MEDIUM/STRONG/EPIC/GOD)
/// * `created_at` - Data di creazione opzionale
/// * `nonce` - Nonce usato per la criptazione AES (12 byte, deve essere UNIQUE)
pub struct StoredPassword {
    pub id: Option<i64>,            // INTEGER PRIMARY KEY,
    pub user_id: i64,               // INTEGER NOT NULL,
    pub location: String,           // TEXT NOT NULL,
    pub password: DbSecretVec,      // TEXT NOT NULL,
    pub notes: Option<String>,      //,
    pub score: PasswordScore,       // integer NOT NULL ,
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
    /// * `score` - Forze della password
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
        password: SecretBox<[u8]>,
        notes: Option<String>,
        score: PasswordScore,
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
            score,
            created_at,
            nonce,
        }
    }
}
#[derive(Debug, Clone)]
pub struct StoredRawPassword {
    pub id: Option<i64>,
    #[allow(unused)]
    pub user_id: i64,
    pub location: String,
    pub password: SecretString,
    pub notes: Option<String>,
    pub score: Option<PasswordScore>,
    pub created_at: Option<String>,
}

impl StoredRawPassword {
    pub fn new() -> Self {
        StoredRawPassword {
            id: None,
            user_id: 0,
            location: "".to_string(),
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
        String,
        SecretString,
        Option<String>,
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
            (Some(id1), Some(id2)) => id1 == id2 && self.location == other.location,
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PasswordEvaluation {
    pub score: Option<PasswordScore>,
    pub reasons: Vec<String>,
}

impl From<PasswordScore> for PasswordEvaluation {
    fn from(score: PasswordScore) -> Self {
        PasswordEvaluation {
            score: Some(score),
            reasons: vec![],
        }
    }
}
impl PasswordEvaluation {
    pub fn strength(&self) -> PasswordStrength {
        match self.score {
            Some(s) => {
                let value = s.value() as i64;
                PasswordScore::get_strength(Some(value))
            }
            None => PasswordStrength::NotEvaluated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
/// Enum che rappresenta la forze della password.
///
/// Viene salvata nel database come testo ('not evaluated','weak', 'medium', 'strong', 'epic', 'god') e
/// controllata da un constraint CHECK.
///
/// # Varianti
///
/// * `NotEvaluated` - Password non valutata
/// * `WEAK` - Password debole
/// * `MEDIUM` - Password media
/// * `STRONG` - Password forte
/// * `EPIC` - Password molto forte
/// * `GOD` - Password molto molto forte
pub enum PasswordStrength {
    NotEvaluated,
    WEAK,
    MEDIUM,
    STRONG,
    EPIC,
    GOD,
}

/// Enum per tenere traccia delle statistiche delle password ( usato nel frontend )
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PasswordStats {
    pub weak: usize,
    pub medium: usize,
    pub strong: usize,
    pub epic: usize,
    pub god: usize,
    pub total: usize,
    pub not_evaluated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type)]
#[sqlx(transparent)]
pub struct PasswordScore(u8);

impl PasswordScore {
    pub const MAX: u8 = 100;

    fn clamp(value: i64) -> u8 {
        let positive = value.max(0); // clamp inferiore
        positive.min(Self::MAX as i64) as u8
    }
    pub fn new<T: Into<i64>>(value: T) -> Self {
        let v = value.into();
        Self(PasswordScore::clamp(v))
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn get_strength(score: Option<i64>) -> PasswordStrength {
        match score {
            Some(s) if s > 95 => PasswordStrength::GOD,
            Some(s) if s >= 85 => PasswordStrength::EPIC,
            Some(s) if s >= 70 => PasswordStrength::STRONG,
            Some(s) if s >= 50 => PasswordStrength::MEDIUM,
            Some(_) => PasswordStrength::WEAK,
            None => PasswordStrength::NotEvaluated,
        }
    }
}

impl PartialEq<u8> for PasswordScore {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for PasswordScore {
    fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for PasswordScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExcludedSymbolSet(HashSet<char>);

impl Type<Sqlite> for ExcludedSymbolSet {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        // Usiamo STRING per dati char in hashset
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, Sqlite> for ExcludedSymbolSet {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s: String = self.0.iter().collect();
        <String as sqlx::Encode<'q, Sqlite>>::encode(s, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ExcludedSymbolSet {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Decodifichiamo come Vec<u8>
        let excluded_symb_string = <String as sqlx::Decode<'r, Sqlite>>::decode(value)?;

        // Convertiamo Vec in SecretBox<[u8]>
        Ok(ExcludedSymbolSet::from(excluded_symb_string))
    }
}
impl From<String> for ExcludedSymbolSet {
    fn from(s: String) -> Self {
        Self(s.chars().filter(|c| !c.is_alphanumeric()).collect())
    }
}

impl std::ops::Deref for ExcludedSymbolSet {
    type Target = HashSet<char>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(sqlx::FromRow, Debug, Clone, Default, SqlxTemplate)]
#[table("passwords_generation_settings")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]

pub struct PasswordGeneratorConfig {
    #[allow(unused)]
    pub id: i64,
    pub settings_id: i64,
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub excluded_symbols: ExcludedSymbolSet,
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_password_score() {
        assert_eq!(PasswordScore::MAX, PasswordScore::new(100).value());
        assert_eq!(PasswordScore::MAX, PasswordScore::new(101).value());
        assert_eq!(PasswordScore::new(0).value(), PasswordScore::new(0).value());
        assert_eq!(
            PasswordScore::new(100).value(),
            PasswordScore::new(1000000).value()
        );
        assert_eq!(
            PasswordScore::new(-100).value(),
            PasswordScore::new(-1000000).value()
        );
        assert_eq!(
            PasswordScore::new(0).value(),
            PasswordScore::new(-1000100).value()
        );
        let ps = PasswordScore::new(86);
        let pe = PasswordEvaluation {
            score: Some(ps),
            reasons: vec![],
        };
        assert_eq!(pe.strength(), PasswordStrength::EPIC);

        let ps = PasswordScore::new(51);
        let pe = PasswordEvaluation {
            score: Some(ps),
            reasons: vec![],
        };
        assert_eq!(pe.strength(), PasswordStrength::MEDIUM);

        let ps = PasswordScore::new(-50);
        let pe = PasswordEvaluation {
            score: Some(ps),
            reasons: vec![],
        };
        assert_eq!(pe.strength(), PasswordStrength::WEAK);

        let ps = PasswordScore::new(50000);
        let pe = PasswordEvaluation {
            score: Some(ps),
            reasons: vec![],
        };
        assert_eq!(pe.strength(), PasswordStrength::GOD);

        let ps = PasswordScore::new(71);
        let pe = PasswordEvaluation {
            score: Some(ps),
            reasons: vec![],
        };
        assert_eq!(pe.strength(), PasswordStrength::STRONG);
        assert!(pe.score.unwrap() > 50);
    }
}
