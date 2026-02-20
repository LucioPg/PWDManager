use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError {
    #[error("Database error: {0}")]
    DBGeneralError(String),
    #[error("Database list error: {0}")]
    DBListError(String),
    #[error("Database select error: {0}")]
    DBSelectError(String),
    #[error("Database delete error: {0}")]
    DBDeleteError(String),
    #[error("Database save error: {0}")]
    DBSaveError(String),
    #[error("Database fetch error: {0}")]
    DBFetchError(String),
    #[error("Database password save error: {0}")]
    DBPasswordSaveError(String),
    #[error("Database password update error: {0}")]
    DBPasswordUpdateError(String),
    #[error("Database password delete error: {0}")]
    DBPasswordDeleteError(String),
    #[error("Database password fetch error: {0}")]
    DBPasswordFetchError(String),
    #[error("Cipher creation error: {0}")]
    DBPCipherCreateError(String),
    #[error("Cipher decryption error: {0}")]
    DBPCipherDecryptError(String),
    #[error("Decrypted password conversion to string error: {0}")]
    DBPasswordConversionError(String),
    #[error("Nonce corruption error")]
    DBNonceCorruptionError(String),
    #[error("Database save temp password error")]
    DBSaveTempPasswordError(String),
    #[error("Cipher encrypt password error")]
    DBCipherEncryptionError(String),
    #[error("Database transaction error: {0}")]
    DBTransactionError(String),
    #[error("Database user settings error: {0}")]
    DBSettingsError(String),
    #[error("Database registration error: {0}")]
    DBRegistrationError(String),
}

impl DBError {
    pub fn new_general_error(msg: String) -> Self {
        DBError::DBGeneralError(msg)
    }

    pub fn new_list_error(msg: String) -> Self {
        DBError::DBListError(msg)
    }

    pub fn new_select_error(msg: String) -> Self {
        DBError::DBSelectError(msg)
    }

    pub fn new_save_error(msg: String) -> Self {
        DBError::DBSaveError(msg)
    }

    pub fn new_delete_error(msg: String) -> Self {
        DBError::DBDeleteError(msg)
    }

    pub fn new_fetch_error(msg: String) -> Self {
        DBError::DBFetchError(msg)
    }

    pub fn new_password_save_error(msg: String) -> Self {
        DBError::DBPasswordSaveError(msg)
    }
    pub fn new_password_update_error(msg: String) -> Self {
        DBError::DBPasswordUpdateError(msg)
    }
    pub fn new_password_delete_error(msg: String) -> Self {
        DBError::DBPasswordDeleteError(msg)
    }
    pub fn new_password_fetch_error(msg: String) -> Self {
        DBError::DBPasswordFetchError(msg)
    }

    pub fn new_cipher_create_error(msg: String) -> Self {
        DBError::DBPCipherCreateError(msg)
    }
    pub fn new_cipher_decrypt_error(msg: String) -> Self {
        DBError::DBPCipherDecryptError(msg)
    }

    pub fn new_password_conversion_error(msg: String) -> Self {
        DBError::DBPasswordConversionError(msg)
    }

    pub fn new_nonce_corruption_error() -> Self {
        DBError::DBNonceCorruptionError("Nonce corrotto".into())
    }
    pub fn new_save_temp_password_error(msg: String) -> Self {
        DBError::DBSaveTempPasswordError(msg.into())
    }

    pub fn new_cipher_encryption_error(msg: String) -> Self {
        DBError::DBCipherEncryptionError(msg.into())
    }

    pub fn new_transaction_error(msg: String) -> Self {
        DBError::DBTransactionError(msg)
    }

    pub fn new_settings_error(msg: String) -> Self {
        DBError::DBSettingsError(msg)
    }

    pub fn new_registration_error(msg: String) -> Self {
        DBError::DBRegistrationError(msg)
    }
}

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

impl EncryptionError {
    pub fn new_encryption_error(msg: String) -> Self {
        EncryptionError::EncryptionError(msg)
    }
}

#[derive(Error, Debug)]
pub enum DecryptionError {
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Password corrotta")]
    RottenPassword(String),
    #[error("Password errata")]
    WrongPassword,
}

impl DecryptionError {
    pub fn new_decryption_error(msg: String) -> Self {
        DecryptionError::DecryptionError(msg)
    }

    pub fn new_rotten_password(msg: String) -> Self {
        DecryptionError::RottenPassword(msg)
    }

    pub fn new_wrong_password() -> Self {
        DecryptionError::WrongPassword
    }
}

#[derive(Error, Debug)]
pub enum AuthGeneralError {
    #[error("Errore nel login")]
    LoginError(String),
    #[error("Errore nel logout")]
    LogoutError,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Errore database: {0}")]
    DB(DBError),
    #[error("Errore di crittografia: {0}")]
    Encryption(EncryptionError),
    #[error("Errore di decrittografia: {0}")]
    Decryption(DecryptionError),
    #[error("Errore di autenticazione: {0}")]
    AuthenticationError(AuthGeneralError),
}

#[derive(Error, Debug)]
pub enum GeneralError {
    #[error("Errore nell'encoding dell'avatar: {0:?}")]
    EncodeError(String),
    #[error("Errore nel ridimensionamento dell'avatar: {0}")]
    ScalingError(String),
}

impl GeneralError {
    pub fn new_encode_error(msg: String) -> Self {
        GeneralError::EncodeError(msg)
    }
    pub fn new_scaling_error(msg: String) -> Self {
        GeneralError::ScalingError(msg)
    }
}
