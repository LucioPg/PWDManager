use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;

#[derive(Clone, PartialEq, Copy)]
pub enum TableOrder {
    AZ,
    ZA,
    Oldest,
    Newest,
}

impl TableOrder {
    /// Ordina un slice di password in-place secondo il criterio selezionato.
    ///
    /// # Arguments
    /// * `passwords` - Slice mutabile di password da ordinare
    pub fn sort(&self, passwords: &mut [StoredRawPassword]) {
        match self {
            TableOrder::AZ => {
                passwords.sort_by(|a, b| a.url.expose_secret().cmp(b.url.expose_secret()))
            }
            TableOrder::ZA => {
                passwords.sort_by(|a, b| b.url.expose_secret().cmp(a.url.expose_secret()))
            }
            TableOrder::Oldest => {
                passwords.sort_by(|a, b| a.created_at.as_ref().cmp(&b.created_at.as_ref()))
            }
            TableOrder::Newest => {
                passwords.sort_by(|a, b| b.created_at.as_ref().cmp(&a.created_at.as_ref()))
            }
        }
    }
}
