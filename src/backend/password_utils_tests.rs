use crate::backend::db_backend::{
    fetch_user_data, get_all_passwords_for_user, init_db, save_or_update_user,
};
use crate::backend::password_utils::{
    calc_strength, create_stored_password_pipeline, decrypt_stored_password,
};
use crate::backend::user_auth_helper::{PasswordStrength, StoredPassword};
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
    let stored_passwords = get_all_passwords_for_user(&pool, user_id)
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
    let strength = calc_strength("abc").await;
    assert_eq!(strength, PasswordStrength::WEAK);
}

#[tokio::test]
async fn test_password_strength_medium() {
    let strength = calc_strength("password123").await;
    assert_eq!(strength, PasswordStrength::MEDIUM);
}

#[tokio::test]
async fn test_password_strength_strong() {
    let strength = calc_strength("veryStrongPassword123!@#").await;
    assert_eq!(strength, PasswordStrength::STRONG);
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

    let mut stored_passwords = get_all_passwords_for_user(&pool, user_id)
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
        PasswordStrength::MEDIUM,
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
    let stored_passwords = get_all_passwords_for_user(&pool, user_id)
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

    let stored_passwords = get_all_passwords_for_user(&pool, user_id)
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
