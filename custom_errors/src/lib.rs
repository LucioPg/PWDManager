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
    DBSaveError(String)
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
}