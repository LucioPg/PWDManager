use crate::backend::db_backend::{
    fetch_user_data, fetch_all_stored_passwords_for_user, init_db, save_or_update_user,
};
use crate::backend::password_types_helper::{
    DbSecretString, PasswordScore, PasswordStrength, StoredPassword, StoredRawPassword, UserAuth,
};
use crate::backend::password_utils::{
    create_cipher, create_stored_password_pipeline, create_stored_passwords,
    decrypt_stored_password,
};
use crate::backend::strength_utils::evaluate_password_strength;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;

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
async fn test_encrypt_decrypt_password() {
    // Setup: inizializza il database
    let pool = init_db().await.expect("Failed to initialize database");

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

    // Verifica che i metadati siano corretti
    assert_eq!(stored_password.location, location);
    assert_eq!(stored_password.notes, notes);
    assert_eq!(stored_password.user_id, user_id);
    assert!(
        !stored_password.nonce.is_empty(),
        "Nonce should not be empty"
    );
    assert_eq!(stored_password.nonce.len(), 12, "Nonce should be 12 bytes");

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
    let pool = init_db().await.expect("Failed to initialize database");

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
            location: locat.to_string(),
            password: rp,
            notes: Some("test rayon".to_string()),
            score: password_evaluation.score,
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

    // Verifica che i metadati siano corretti
    assert_eq!(stored_password.location, location);
    assert_eq!(stored_password.notes, notes);
    assert_eq!(stored_password.user_id, user_id);
    assert!(
        !stored_password.nonce.is_empty(),
        "Nonce should not be empty"
    );
    assert_eq!(stored_password.nonce.len(), 12, "Nonce should be 12 bytes");

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
    let pool = init_db().await.expect("Failed to initialize database");

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
    stored_password.nonce = vec![1, 2, 3]; // Solo 3 byte invece di 12

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
    let pool = init_db().await.expect("Failed to initialize database");

    // Crea una stored password con un user_id inesistente
    let stored_password = StoredPassword::new(
        None,
        99999, // User ID inesistente
        "https://fake.com".to_string(),
        SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()),
        None,
        PasswordScore::new(38),
        None,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    );

    let result = decrypt_stored_password(&pool, &stored_password).await;
    assert!(result.is_err(), "Should fail with nonexistent user");
}

#[tokio::test]
async fn test_multiple_passwords_for_same_user() {
    let pool = init_db().await.expect("Failed to initialize database");

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
    for (i, (expected_location, expected_password)) in passwords.iter().enumerate() {
        let stored = &stored_passwords[i];
        assert_eq!(stored.location, *expected_location);

        let decrypted = decrypt_stored_password(&pool, stored)
            .await
            .expect("Failed to decrypt password");
        assert_eq!(decrypted, *expected_password);
    }
}

#[tokio::test]
async fn test_multiple_passwords_for_same_user_with_predefined_strength() {
    let pool = init_db().await.expect("Failed to initialize database");

    let user_id = create_test_user(&pool, "testuser3", "MasterPass456!").await;

    // Crea più password per lo stesso utente
    let passwords = vec![
        (
            "https://site1.com-strong",
            "Password1",
            PasswordScore::new(70),
        ),
        (
            "https://site2.com-medium",
            "Password2",
            PasswordScore::new(51),
        ),
        (
            "https://site3.com-weak",
            "VeryLongSecurePassword123!",
            PasswordScore::new(10),
        ),
    ];

    for (location, raw_pwd, strength) in &passwords {
        create_stored_password_pipeline(
            &pool,
            user_id,
            location.to_string(),
            SecretString::new(raw_pwd.to_string().into()),
            None,
            Some(strength.to_owned()),
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
    for (i, (expected_location, expected_password, expected_strength)) in
        passwords.iter().enumerate()
    {
        let stored = &stored_passwords[i];
        assert_eq!(stored.location, *expected_location);
        assert_eq!(stored.score, expected_strength.clone());
        let decrypted = decrypt_stored_password(&pool, stored)
            .await
            .expect("Failed to decrypt password");
        assert_eq!(decrypted, *expected_password);
    }
}

#[tokio::test]
async fn test_encrypted_password_is_different_from_original() {
    let pool = init_db().await.expect("Failed to initialize database");

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
