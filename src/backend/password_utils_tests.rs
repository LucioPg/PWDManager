use crate::backend::db_backend::{
    create_user_settings, fetch_all_stored_passwords_for_user, fetch_user_data,
    fetch_user_passwords_generation_settings, save_or_update_user,
};
use pwd_types::{
    DbSecretString, PasswordPreset, PasswordScore, PasswordStrength, StoredPassword,
    StoredRawPassword, UserAuth,
};
use crate::backend::password_utils::{
    create_cipher, create_stored_password_pipeline, create_stored_passwords,
    decrypt_stored_password, generate_suggested_password,
};
use crate::backend::evaluate_password_strength;
use crate::backend::test_helpers::setup_test_db;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;
use std::time::Instant;

/// Helper function per creare un utente di test e restituire l'ID
/// Usa un timestamp per garantire username univoci tra i test
async fn create_test_user(pool: &SqlitePool, base_username: &str, password: &str) -> i64 {
    // Genera un username univoco usando il timestamp attuale
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let username = format!("{}_{}", base_username, timestamp);

    let password = SecretString::new(password.into());
    save_or_update_user(pool, None, username.clone(), Some(password), None)
        .await
        .expect("Failed to create test user");

    // Recupera l'ID dell'utente appena creato
    let (user_id, _, _, _) = fetch_user_data(pool, &username)
        .await
        .expect("Failed to fetch test user");
    user_id
}

#[tokio::test]
async fn test_generate_password_from_preset() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser", "MasterPass123!").await;
    let setting_id = create_user_settings(&pool, user_id, PasswordPreset::God)
        .await
        .unwrap();
    let mut pass_settings = fetch_user_passwords_generation_settings(&pool, user_id)
        .await
        .unwrap();
    let presets: Vec<(PasswordPreset, PasswordStrength)> = vec![
        (PasswordPreset::Epic, PasswordStrength::EPIC),
        (PasswordPreset::Strong, PasswordStrength::STRONG),
        (PasswordPreset::Medium, PasswordStrength::MEDIUM),
    ];
    let start = Instant::now();
    let password = generate_suggested_password(Some(pass_settings.clone()));
    let duration = start.elapsed();
    println!(
        "Generated password: {} in {}",
        password.expose_secret(),
        duration.as_secs()
    );
    let evaluated_password = evaluate_password_strength(&password, None);
    assert_eq!(evaluated_password.strength(), PasswordStrength::GOD);
    for preset in presets {
        let start = Instant::now();
        let password = generate_suggested_password(Some(preset.0.to_config(setting_id)));
        let duration = start.elapsed();
        println!(
            "preset-{} Generated password: {} in {}",
            preset.0,
            password.expose_secret(),
            duration.as_secs()
        );
        let evaluated_password = evaluate_password_strength(&password, None);
        assert_eq!(evaluated_password.strength(), preset.1);
    }
}

#[tokio::test]
async fn test_encrypt_decrypt_password() {
    // Setup: inizializza il database
    let pool = setup_test_db().await;

    // Crea un utente di test
    let master_password = "MasterPass123!";
    let user_id = create_test_user(&pool, "testuser", master_password).await;

    // Test 1: Cifra una password
    let raw_password = SecretString::new("MySecurePassword456".into());
    let location = "https://example.com".to_string();
    let notes = Some("Test password".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt password");

    // Test 2: Recupera la password cifrata
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(
        stored_passwords.len(),
        1,
        "Should have exactly one stored password"
    );
    let stored_password = &stored_passwords[0];

    // Verifica che location sia crittografato (non plain text)
    let location_bytes: &[u8] = stored_password.location.expose_secret().as_ref();
    let location_plain: &[u8] = location.as_bytes();
    assert_ne!(
        location_bytes, location_plain,
        "Location should be encrypted"
    );

    // Verifica che notes siano crittografate (non plain text)
    if let Some(notes_enc) = &stored_password.notes {
        let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
        let notes_plain: &[u8] = notes.as_ref().unwrap().as_bytes();
        assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
    }

    assert_eq!(stored_password.user_id, user_id);

    // Verifica i nonce (password_nonce e location_nonce devono essere 12 byte)
    assert!(
        !stored_password.password_nonce.is_empty(),
        "Password nonce should not be empty"
    );
    assert_eq!(
        stored_password.password_nonce.len(),
        12,
        "Password nonce should be 12 bytes"
    );
    assert_eq!(
        stored_password.location_nonce.len(),
        12,
        "Location nonce should be 12 bytes"
    );
    if let Some(nn) = &stored_password.notes_nonce {
        assert_eq!(nn.len(), 12, "Notes nonce should be 12 bytes");
    }

    // Test 3: Decifra la password
    let decrypted_password = decrypt_stored_password(&pool, stored_password)
        .await
        .expect("Failed to decrypt password");

    assert_eq!(
        decrypted_password,
        raw_password.expose_secret(),
        "Decrypted password should match original"
    );
}

#[tokio::test]
async fn test_encrypt_decrypt_password_rayon() {
    // Setup: inizializza il database
    let pool = setup_test_db().await;

    // Crea un utente di test
    let master_password = "MasterPass123!";
    let user_id = create_test_user(&pool, "testuser", master_password).await;
    let user_auth = UserAuth {
        id: user_id,
        password: DbSecretString(SecretString::from(master_password)),
    };
    // Test 1: Cifra una password
    let raw_password = SecretString::new("MySecurePassword456".into());
    let location = "https://example.com".to_string();
    let notes = Some("Test password".to_string());
    let salt = crate::backend::utils::generate_salt();
    let cipher = create_cipher(&salt.as_salt(), &user_auth);
    let data: Vec<_> = vec![
        (
            "https://site1.com-strong",
            "Password1",
            evaluate_password_strength(&SecretString::new("Password1".into()), None),
        ),
        (
            "https://site2.com-medium",
            "Password2",
            evaluate_password_strength(&SecretString::new("PAssword2".into()), None),
        ),
        (
            "https://site3.com-weak",
            "VeryLongSecurePassword123!",
            evaluate_password_strength(
                &SecretString::new("VeryLongSecurePassword123!".into()),
                None,
            ),
        ),
    ];
    let mut stored_raw_passwords: Vec<StoredRawPassword> = vec![];
    for (locat, raw_pwd, password_evaluation) in &data {
        let rp = SecretString::new(raw_pwd.to_owned().into());
        let stored_password_raw = StoredRawPassword {
            id: None,
            user_id,
            location: SecretString::new(locat.to_string().into()),
            password: rp,
            notes: Some(SecretString::new("test rayon".into())),
            score: password_evaluation.score,
            created_at: None,
        };
        stored_raw_passwords.push(stored_password_raw)
    }

    let result = create_stored_passwords(cipher.unwrap(), user_auth, stored_raw_passwords).await;
    match result {
        Ok(sp) => {
            assert_eq!(
                sp.len(),
                data.len(),
                "Should have exactly same number of items"
            );
            for s in sp {
                println!("{:?}", s);
            }
        }
        Err(e) => panic!("Failed to create stored passwords: {:?}", e),
    }

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt password");

    // Test 2: Recupera la password cifrata
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(
        stored_passwords.len(),
        1,
        "Should have exactly one stored password"
    );
    let stored_password = &stored_passwords[0];

    // Verifica che location sia crittografato (non plain text)
    let location_bytes: &[u8] = stored_password.location.expose_secret().as_ref();
    let location_plain: &[u8] = location.as_bytes();
    assert_ne!(
        location_bytes, location_plain,
        "Location should be encrypted"
    );

    // Verifica che notes siano crittografate
    if let Some(notes_enc) = &stored_password.notes {
        let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
        let notes_plain: &[u8] = notes.as_ref().unwrap().as_bytes();
        assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
    }

    assert_eq!(stored_password.user_id, user_id);

    // Verifica i nonce
    assert!(
        !stored_password.password_nonce.is_empty(),
        "Password nonce should not be empty"
    );
    assert_eq!(
        stored_password.password_nonce.len(),
        12,
        "Password nonce should be 12 bytes"
    );
    assert_eq!(
        stored_password.location_nonce.len(),
        12,
        "Location nonce should be 12 bytes"
    );

    // Test 3: Decifra la password
    let decrypted_password = decrypt_stored_password(&pool, stored_password)
        .await
        .expect("Failed to decrypt password");

    assert_eq!(
        decrypted_password,
        raw_password.expose_secret(),
        "Decrypted password should match original"
    );
}

#[tokio::test]
async fn test_password_strength_weak() {
    let evaluation = evaluate_password_strength(&SecretString::new("abc".into()), None);
    assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
    // La password è troppo corta, quindi dovrebbe avere reasons
    assert!(!evaluation.reasons.is_empty());
    assert!(evaluation.score.is_some());
    // Score dovrebbe essere basso
    assert!(evaluation.score.unwrap() < 50);
}

#[tokio::test]
async fn test_password_strength_medium() {
    // "password123" è nella blacklist, quindi uso una password diversa
    // "MyPass123!" ha tutti i 4 tipi di caratteri e lunghezza 10
    // Score: 5 (length) + 60 (4 variety) + 0 bonus = 65 -> MEDIUM
    let evaluation = evaluate_password_strength(&SecretString::new("MyPass123!".into()), None);
    assert_eq!(evaluation.strength(), PasswordStrength::MEDIUM);
    assert!(evaluation.score.is_some());
    // Score MEDIUM è tra 50 e 69
    let score = evaluation.score.unwrap();
    assert!(
        score >= 50 && score < 70,
        "Expected MEDIUM score (50-69), got {}",
        score
    );
}

#[tokio::test]
async fn test_password_strength_strong() {
    let evaluation =
        evaluate_password_strength(&SecretString::new("veryStrongPassword123!@#".into()), None);
    // Con il nuovo sistema, questa password dovrebbe essere STRONG o superiore
    // Verifica usando lo score (>= 70 per STRONG, >= 85 per EPIC, > 95 per GOD)
    assert!(
        matches!(
            evaluation.strength(),
            PasswordStrength::STRONG | PasswordStrength::EPIC | PasswordStrength::GOD
        ),
        "Expected STRONG or better, got {:?}",
        evaluation.strength()
    );
    assert!(evaluation.score.is_some());
    assert!(evaluation.score.unwrap() >= 70);
}

#[tokio::test]
async fn test_decrypt_invalid_nonce() {
    let pool = setup_test_db().await;

    let user_id = create_test_user(&pool, "testuser2", "MasterPass123!").await;

    // Crea una password valida
    let raw_password = SecretString::new("TestPassword".into());
    create_stored_password_pipeline(
        &pool,
        user_id,
        "https://test.com".to_string(),
        raw_password,
        None,
        None,
    )
    .await
    .expect("Failed to encrypt password");

    let mut stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");
    let mut stored_password = stored_passwords.pop().unwrap();

    // Corrompi il nonce (lunghezza errata)
    stored_password.password_nonce = vec![1, 2, 3]; // Solo 3 byte invece di 12

    let result = decrypt_stored_password(&pool, &stored_password).await;
    assert!(result.is_err(), "Should fail with invalid nonce length");

    if let Err(custom_errors::DBError::DBNonceCorruptionError(_)) = result {
        // Ok, errore atteso
    } else {
        panic!("Expected DBNonceCorruptionError, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_decrypt_nonexistent_user() {
    let pool = setup_test_db().await;

    // Crea una stored password con un user_id inesistente
    let stored_password = StoredPassword::new(
        None,
        99999, // User ID inesistente
        SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // encrypted location
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // location_nonce
        SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // password
        None,  // notes
        None,  // notes_nonce
        PasswordScore::new(38),
        None,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // password_nonce
    );

    let result = decrypt_stored_password(&pool, &stored_password).await;
    assert!(result.is_err(), "Should fail with nonexistent user");
}

#[tokio::test]
async fn test_multiple_passwords_for_same_user() {
    let pool = setup_test_db().await;

    let user_id = create_test_user(&pool, "testuser3", "MasterPass456!").await;

    // Crea più password per lo stesso utente
    let passwords = vec![
        ("https://site1.com", "Password1"),
        ("https://site2.com", "Password2"),
        ("https://site3.com", "VeryLongSecurePassword123!"),
    ];

    for (location, raw_pwd) in &passwords {
        create_stored_password_pipeline(
            &pool,
            user_id,
            location.to_string(),
            SecretString::new(raw_pwd.to_string().into()),
            None,
            None,
        )
        .await
        .expect("Failed to encrypt password");
    }

    // Recupera tutte le password
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(stored_passwords.len(), 3, "Should have 3 stored passwords");

    // Verifica che ogni password possa essere decifrata correttamente
    // Nota: location è crittografato, quindi non confrontiamo direttamente
    for (i, (_expected_location, expected_password)) in passwords.iter().enumerate() {
        let stored = &stored_passwords[i];

        let decrypted = decrypt_stored_password(&pool, stored)
            .await
            .expect("Failed to decrypt password");
        assert_eq!(decrypted, *expected_password);
    }
}

#[tokio::test]
async fn test_multiple_passwords_for_same_user_with_predefined_strength() {
    // Initialize blacklist for accurate password evaluation
    let _ = crate::backend::init_blacklist();

    let pool = setup_test_db().await;

    let user_id = create_test_user(&pool, "t", "t").await;

    // Crea più password per lo stesso utente
    let passwords = vec![
        (
            "https://site1.com-weak",
            "ciaociao",
            evaluate_password_strength(&SecretString::new("ciaociao".into()), None),
        ),
        (
            "https://site2.com-medium",
            "Password2!",
            evaluate_password_strength(&SecretString::new("Password2!".into()), None),
        ),
        (
            "https://site3.com-epic",
            "VeryLongSecurePassword123!",
            evaluate_password_strength(
                &SecretString::new("VeryLongSecurePassword123!".into()),
                None,
            ),
        ),
    ];

    for (location, raw_pwd, strength) in &passwords {
        create_stored_password_pipeline(
            &pool,
            user_id,
            location.to_string(),
            SecretString::new(raw_pwd.to_string().into()),
            Some("Ciao io sono una nota".to_string()),
            strength.score,
        )
        .await
        .expect("Failed to encrypt password");
    }

    // Recupera tutte le password
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(stored_passwords.len(), 3, "Should have 3 stored passwords");

    // Verifica che ogni password possa essere decifrata correttamente
    // Nota: location è crittografato, quindi non confrontiamo direttamente
    for (i, (_expected_location, expected_password, expected_strength)) in
        passwords.iter().enumerate()
    {
        let stored = &stored_passwords[i];
        let expected_strength_score = expected_strength.score.map(|s| s.value()).unwrap_or(0);
        assert_eq!(stored.score.value(), expected_strength_score);
        let decrypted = decrypt_stored_password(&pool, stored)
            .await
            .expect("Failed to decrypt password");
        assert_eq!(decrypted, *expected_password);
    }
}

#[tokio::test]
async fn test_encrypted_password_is_different_from_original() {
    let pool = setup_test_db().await;

    let user_id = create_test_user(&pool, "testuser4", "MasterPass789!").await;

    let raw_password = "MyPassword123";
    create_stored_password_pipeline(
        &pool,
        user_id,
        "https://encrypted.com".to_string(),
        SecretString::new(raw_password.into()),
        None,
        None,
    )
    .await
    .expect("Failed to encrypt password");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");
    let stored_password = &stored_passwords[0];

    // La password cifrata nel database NON dovrebbe essere uguale alla password in chiaro
    let encrypted_bytes: &[u8] = stored_password.password.expose_secret().as_ref();
    let raw_password_bytes = raw_password.as_bytes();

    assert_ne!(
        encrypted_bytes, raw_password_bytes,
        "Encrypted bytes should differ from original password bytes"
    );

    // Ma dopo la decifrazione dovrebbe essere uguale
    let decrypted = decrypt_stored_password(&pool, stored_password)
        .await
        .expect("Failed to decrypt password");
    assert_eq!(decrypted, raw_password);
}

#[tokio::test]
async fn test_location_and_notes_are_encrypted() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_enc", "MasterPass123!").await;

    let raw_password = SecretString::new("MySecurePassword456".into());
    let location = "https://secret-location.com".to_string();
    let notes = Some("Confidential notes".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt data");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    let stored = &stored_passwords[0];

    // Verify location is NOT plaintext
    let location_bytes: &[u8] = stored.location.expose_secret().as_ref();
    let location_plain: &[u8] = location.as_bytes();
    assert_ne!(
        location_bytes, location_plain,
        "Location should be encrypted"
    );

    // Verify notes are NOT plaintext
    if let Some(notes_enc) = &stored.notes {
        let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
        let notes_plain: &[u8] = notes.as_ref().unwrap().as_bytes();
        assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
    }

    // Verify nonces are 12 bytes
    assert_eq!(stored.location_nonce.len(), 12);
    assert_eq!(stored.password_nonce.len(), 12);
    if let Some(nn) = &stored.notes_nonce {
        assert_eq!(nn.len(), 12);
    }
}

#[tokio::test]
async fn test_decrypt_location_and_notes_roundtrip() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_rt", "MasterPass123!").await;

    let raw_password = SecretString::new("MyPassword".into());
    let location = "MySecretService".to_string();
    let notes = Some("My secret notes".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt");

    // Usa get_stored_raw_passwords che recupera correttamente l'UserAuth dal DB
    let decrypted = crate::backend::password_utils::get_stored_raw_passwords(&pool, user_id)
        .await
        .expect("Failed to decrypt");

    assert_eq!(decrypted.len(), 1);
    assert_eq!(decrypted[0].location.expose_secret(), location);
    // Confronta notes: Option<SecretString> vs Option<String>
    match (&decrypted[0].notes, &notes) {
        (Some(dec_notes), Some(exp_notes)) => {
            assert_eq!(dec_notes.expose_secret(), exp_notes);
        }
        (None, None) => {}
        _ => panic!(
            "Notes mismatch: expected {:?}, got {:?}",
            notes, decrypted[0].notes
        ),
    }
    assert_eq!(
        decrypted[0].password.expose_secret(),
        raw_password.expose_secret()
    );
}
