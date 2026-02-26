//! Tipi per la valutazione della forza delle password.

use std::fmt;

/// Rappresenta un punteggio password da 0 a 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
pub struct PasswordScore(u8);

impl PasswordScore {
    pub const MAX: u8 = 100;

    fn clamp(value: i64) -> u8 {
        let positive = value.max(0);
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

/// Enum che rappresenta la forza della password.
///
/// Viene salvata nel database come testo ('not evaluated','weak', ecc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "TEXT", rename_all = "lowercase"))]
pub enum PasswordStrength {
    NotEvaluated,
    WEAK,
    MEDIUM,
    STRONG,
    EPIC,
    GOD,
}

/// Risultato della valutazione di una password.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_score_clamp() {
        assert_eq!(PasswordScore::MAX, PasswordScore::new(100).value());
        assert_eq!(PasswordScore::MAX, PasswordScore::new(101).value());
        assert_eq!(0, PasswordScore::new(-100).value());
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(PasswordStrength::GOD, PasswordScore::get_strength(Some(96)));
        assert_eq!(PasswordStrength::EPIC, PasswordScore::get_strength(Some(85)));
        assert_eq!(PasswordStrength::STRONG, PasswordScore::get_strength(Some(70)));
        assert_eq!(PasswordStrength::MEDIUM, PasswordScore::get_strength(Some(50)));
        assert_eq!(PasswordStrength::WEAK, PasswordScore::get_strength(Some(10)));
        assert_eq!(PasswordStrength::NotEvaluated, PasswordScore::get_strength(None));
    }
}
