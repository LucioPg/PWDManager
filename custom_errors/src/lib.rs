use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError {
    #[error("Database error")]
    DBGeneralError(String),
    #[error("Database list error")]
    DBListError(String),
    #[error("Database select error")]
    DBSelectError(String)
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
}