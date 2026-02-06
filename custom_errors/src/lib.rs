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
    }

impl DBError {
    pub fn new_general_error(msg: String) -> Self {
        DBError::DBGeneralError(msg.into())
    }

    pub fn new_list_error(msg: String) -> Self {
        DBError::DBListError(msg.into())
    }

    pub fn new_select_error(msg: String) -> Self {
        DBError::DBSelectError(msg.into())
    }

    pub fn new_save_error(msg: String) -> Self {
        DBError::DBSaveError(msg.into())
    }

    pub fn new_delete_error(msg: String) -> Self {
        DBError::DBDeleteError(msg.into())
    }
    
    pub fn new_fetch_error(msg: String) -> Self {
        DBError::DBFetchError(msg.into())
    }
}

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption error: {0}")]
    EncryptionError(String)
}

impl EncryptionError {
    pub fn new_encryption_error(msg: String) -> Self {
        EncryptionError::EncryptionError(msg.into())
    }
}

#[derive(Error, Debug)]
pub enum DecryptionError {
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Password corrotta")]
    RottenPassword(String),
    #[error("Password errata")]
    WrongPassword
}

impl DecryptionError {
    pub fn new_decryption_error(msg: String) -> Self {
        DecryptionError::DecryptionError(msg.into())
    }
    
    pub fn new_rotten_password(msg: String) -> Self {
        DecryptionError::RottenPassword(msg.into())
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
    LogoutError
}

#[derive(Debug)]
pub enum AuthError {
    DB(DBError),
    Encryption(EncryptionError),
    Decryption(DecryptionError),
    AuthenticationError(AuthGeneralError)
}