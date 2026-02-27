//! Risultato del cambiamento password per callback UI.
//!
//! Questo tipo è usato da `PasswordHandler` per notificare il consumer
//! quando la password cambia, includendo tutti i metadati della valutazione.

use crate::{PasswordScore, PasswordStrength};
use secrecy::SecretString;

/// Risultato di un cambio password, passato ai callback `on_password_change`.
///
/// Contiene la password segreta e tutti i metadati della sua valutazione.
///
/// # Example
///
/// ```rust,ignore
/// use pwd_types::PasswordChangeResult;
/// use secrecy::SecretString;
///
/// let result = PasswordChangeResult {
///     password: SecretString::new("my_secure_password".into()),
///     score: Some(PasswordScore::new(85)),
///     strength: PasswordStrength::EPIC,
///     reasons: vec!["Length OK".to_string()],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PasswordChangeResult {
    /// La password segreta (wrapped in SecretString)
    pub password: SecretString,
    /// Punteggio della password (0-100), None se non valutata
    pub score: Option<PasswordScore>,
    /// Classificazione della forza
    pub strength: PasswordStrength,
    /// Ragioni della valutazione (per tooltip/info)
    pub reasons: Vec<String>,
}

impl PasswordChangeResult {
    /// Crea un nuovo risultato con password non valutata.
    pub fn new(password: SecretString) -> Self {
        Self {
            password,
            score: None,
            strength: PasswordStrength::NotEvaluated,
            reasons: Vec::new(),
        }
    }

    /// Crea un risultato da una valutazione esistente.
    pub fn from_evaluation(
        password: SecretString,
        evaluation: crate::PasswordEvaluation,
    ) -> Self {
        let strength = evaluation.strength();
        Self {
            password,
            score: evaluation.score,
            strength,
            reasons: evaluation.reasons,
        }
    }

    /// Verifica se la password è stata valutata.
    pub fn is_evaluated(&self) -> bool {
        self.score.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_result() {
        let result = PasswordChangeResult::new(SecretString::new("test".into()));
        assert!(result.score.is_none());
        assert_eq!(result.strength, PasswordStrength::NotEvaluated);
        assert!(!result.is_evaluated());
    }

    #[test]
    fn test_from_evaluation() {
        let evaluation = crate::PasswordEvaluation::from(PasswordScore::new(85));
        let result =
            PasswordChangeResult::from_evaluation(SecretString::new("password".into()), evaluation);

        assert!(result.score.is_some());
        assert_eq!(result.strength, PasswordStrength::EPIC);
        assert!(result.is_evaluated());
    }
}
