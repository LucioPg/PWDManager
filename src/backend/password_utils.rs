use custom_errors::DBError;
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;
use tracing::debug;

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum PasswordStrength {
    WEAK,
    MEDIUM,
    STRONG,
}

fn calc_strength(password: &str) -> PasswordStrength {
    if password.len() < 8 {
        return PasswordStrength::WEAK;
    };
    if password.len() >= 8 && password.len() < 16 {
        return PasswordStrength::MEDIUM;
    };
    PasswordStrength::STRONG
}

/*
esempio per usare la conversione enum -> text di sqlx
sqlx::query!(
    "INSERT INTO users (name, strength) VALUES (?1, ?2)",
    "Lucio",
    Strength::Strong as Strength
)
.execute(&pool)
.await?;
 */

pub async fn save_or_update_password(
    pool: &SqlitePool,
    id: Option<i32>, // Se Some, fa l'UPDATE. Se None, fa l'INSERT.
    user_id: i32,
    location: String,
    password: SecretString,
    notes: Option<String>,
    strength: PasswordStrength,
) -> dioxus::Result<(), DBError> {
    debug!("Attempting to save/update user password");

    // 1. Criptazione comune a entrambi i casi

    match id {
        // --- CASO UPDATE ---
        Some(id) => {
            if !password.expose_secret().trim().is_empty() && !location.trim().is_empty() {
                let password_clone = password.clone();
                let hash_password =
                    crate::backend::utils::encrypt(password_clone).map_err(|e| {
                        DBError::new_password_save_error(format!("Failed to encrypt: {}", e))
                    })?;
                let password_strength = calc_strength(&password.expose_secret());
                sqlx::query("UPDATE passwords SET location = ?, password = ?, strength = ?, notes = ? WHERE id = ? AND user_di = ?")
                    .bind(location)
                    .bind(hash_password)
                    .bind(password_strength)
                    .bind(notes)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| DBError::new_password_save_error(format!("Update failed: {}", e)))?;
            }
        }
        // _ => {
        //     todo!("completare la logica per l'inserimento di una nuova password")
        //         // sqlx::query("UPDATE users SET username = ?, avatar = ? WHERE id = ?")
        //         //     .bind(username)
        //         //     .bind(avatar)
        //         //     .bind(user_id)
        //         //     .execute(pool)
        //         //     .await
        //         //     .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;
        //     }
        // },
        // --- CASO INSERT ---
        None => {
            todo!("completare la logica per l'inserimento di una nuova password")
            // let psw = password.unwrap_or_default();
            // let hash_password = crate::backend::utils::encrypt(&psw)
            //     .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
            // sqlx::query("INSERT INTO users (username, password, avatar) VALUES (?, ?, ?)")
            //     .bind(username)
            //     .bind(hash_password)
            //     .bind(avatar)
            //     .execute(pool)
            //     .await
            //     .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;
        }
    }

    Ok(())
}
