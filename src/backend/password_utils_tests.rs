use crate::backend::db_backend::{
    create_user_settings, fetch_all_stored_passwords_for_user, fetch_user_auth_from_id,
    fetch_user_data, fetch_user_passwords_generation_settings, fetch_user_temp_old_password,
    save_or_update_user,
};
use crate::backend::evaluate_password_strength;
use crate::backend::password_utils::{
    create_stored_data_pipeline_bulk, decrypt_bulk_stored_data,
    generate_suggested_password, get_stored_raw_passwords,
    stored_passwords_migration_pipeline_with_progress,
};
use crate::backend::test_helpers::setup_test_db;
use pwd_types::{
    ExcludedSymbolSet, PasswordGeneratorConfig, PasswordPreset, PasswordScore,
    PasswordStrength, StoredRawPassword,
};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;
use std::time::Instant;
use uuid::Uuid;

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
    let url = "https://example.com".to_string();
    let notes = Some(SecretString::new("Test password".into()));
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new(url.to_string().into()),
        password: raw_password.clone(),
        notes: notes.clone(),
        score: None,
        created_at: None,
    };
    let stored_raw_passwords = vec![stored_raw_password];
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
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

    // Verifica che url sia crittografato (non plain text)
    let url_bytes: &[u8] = stored_password.url.expose_secret().as_ref();
    let url_plain: &[u8] = url.as_bytes();
    assert_ne!(url_bytes, url_plain, "url should be encrypted");

    // Verifica che notes siano crittografate (non plain text)
    if let Some(notes_enc) = &stored_password.notes {
        let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
        let notes_plain: &[u8] = notes.as_ref().unwrap().expose_secret().as_bytes();
        assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
    }

    assert_eq!(stored_password.user_id, user_id);

    // Verifica i nonce (password_nonce e url_nonce devono essere 12 byte)
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
        stored_password.url_nonce.len(),
        12,
        "url nonce should be 12 bytes"
    );
    if let Some(nn) = &stored_password.notes_nonce {
        assert_eq!(nn.len(), 12, "Notes nonce should be 12 bytes");
    }

    // Test 3: Decifra la password
    let user_auth = fetch_user_auth_from_id(&pool, user_id).await.unwrap();
    let decrypted_raw = decrypt_bulk_stored_data(user_auth, vec![stored_password.clone()], None)
        .await
        .expect("Failed to decrypt password");
    let decrypted_password = decrypted_raw[0].password.expose_secret().to_string();

    assert_eq!(
        decrypted_password,
        raw_password.expose_secret(),
        "Decrypted password should match original"
    );
}

// ATTENZIONE NON CANCELLARE!!
// Questo test è stato commentato perché FLAKY
// Va riattivato quando si trova il modo di fare solo questo test specifico, invece che tutti insieme
// #[tokio::test]
// async fn test_encrypt_decrypt_password_rayon() {
//     // Setup: inizializza il database
//     let pool = setup_test_db().await;
//
//     // Crea un utente di test
//     let master_password = "MasterPass123!";
//     let user_id = create_test_user(&pool, "testuser", master_password).await;
//     let user_auth = UserAuth {
//         id: user_id,
//         password: DbSecretString(SecretString::from(master_password)),
//     };
//     // Test 1: Cifra una password
//     let raw_password = SecretString::new("MySecurePassword456".into());
//     let url = "https://example.com".to_string();
//     let notes = Some("Test password".to_string());
//     let salt = crate::backend::utils::generate_salt();
//     let cipher = create_cipher(&salt.as_salt(), &user_auth);
//     let data: Vec<_> = vec![
//         (
//             "https://site1.com-strong",
//             "Password1",
//             evaluate_password_strength(&SecretString::new("Password1".into()), None),
//         ),
//         (
//             "https://site2.com-medium",
//             "Password2",
//             evaluate_password_strength(&SecretString::new("PAssword2".into()), None),
//         ),
//         (
//             "https://site3.com-weak",
//             "VeryLongSecurePassword123!",
//             evaluate_password_strength(
//                 &SecretString::new("VeryLongSecurePassword123!".into()),
//                 None,
//             ),
//         ),
//     ];
//     let mut stored_raw_passwords: Vec<StoredRawPassword> = vec![];
//     for (locat, raw_pwd, password_evaluation) in &data {
//         let rp = SecretString::new(raw_pwd.to_owned().into());
//         let stored_password_raw = StoredRawPassword {
//             uuid: Uuid::new_v4(),
//             id: None,
//             user_id,
//             url: SecretString::new(locat.to_string().into()),
//             password: rp,
//             notes: Some(SecretString::new("test rayon".into())),
//             score: password_evaluation.score,
//             created_at: None,
//         };
//         stored_raw_passwords.push(stored_password_raw)
//     }
//
//     let result =
//         create_stored_data_records(cipher.unwrap(), user_auth, stored_raw_passwords.clone()).await;
//     match result {
//         Ok(sp) => {
//             assert_eq!(
//                 sp.len(),
//                 data.len(),
//                 "Should have exactly same number of items"
//             );
//             for s in sp {
//                 println!("{:?}", s);
//             }
//         }
//         Err(e) => panic!("Failed to create stored passwords: {:?}", e),
//     };
//     create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
//         .await
//         .expect("Failed to encrypt password");
//
//     // Test 2: Recupera la password cifrata
//     let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
//         .await
//         .expect("Failed to fetch stored passwords");
//
//     assert_eq!(
//         stored_passwords.len(),
//         1,
//         "Should have exactly one stored password"
//     );
//     let stored_password = &stored_passwords[0];
//
//     // Verifica che url sia crittografato (non plain text)
//     let url_bytes: &[u8] = stored_password.url.expose_secret().as_ref();
//     let url_plain: &[u8] = url.as_bytes();
//     assert_ne!(
//         url_bytes, url_plain,
//         "url should be encrypted"
//     );
//
//     // Verifica che notes siano crittografate
//     if let Some(notes_enc) = &stored_password.notes {
//         let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
//         let notes_plain: &[u8] = notes.as_ref().unwrap().as_bytes();
//         assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
//     }
//
//     assert_eq!(stored_password.user_id, user_id);
//
//     // Verifica i nonce
//     assert!(
//         !stored_password.password_nonce.is_empty(),
//         "Password nonce should not be empty"
//     );
//     assert_eq!(
//         stored_password.password_nonce.len(),
//         12,
//         "Password nonce should be 12 bytes"
//     );
//     assert_eq!(
//         stored_password.url_nonce.len(),
//         12,
//         "url nonce should be 12 bytes"
//     );
//
//     // Test 3: Decifra la password
//     let decrypted_password = decrypt_stored_password(&pool, stored_password)
//         .await
//         .expect("Failed to decrypt password");
//
//     assert_eq!(
//         decrypted_password,
//         raw_password.expose_secret(),
//         "Decrypted password should match original"
//     );
// }

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
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new("https://test.com".to_string().into()),
        password: raw_password,
        notes: None,
        score: None,
        created_at: None,
    };
    let stored_raw_passwords = vec![stored_raw_password];
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to encrypt password");

    let mut stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");
    let mut stored_password = stored_passwords.pop().unwrap();

    // Corrompi il nonce (lunghezza errata)
    stored_password.password_nonce = vec![1, 2, 3]; // Solo 3 byte invece di 12

    let user_auth = fetch_user_auth_from_id(&pool, user_id).await.unwrap();
    let result = decrypt_bulk_stored_data(user_auth, vec![stored_password], None).await;
    assert!(result.is_err(), "Should fail with invalid nonce length");

    if let Err(custom_errors::DBError::DBNonceCorruptionError(_)) = result {
        // Ok, errore atteso
    } else {
        panic!("Expected DBNonceCorruptionError, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_decrypt_with_wrong_key() {
    let pool = setup_test_db().await;

    // Crea un utente e cripta una password con la sua chiave
    let user_id = create_test_user(&pool, "wrongkey_test", "CorrectPassword123!").await;

    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: secrecy::SecretString::new(String::new().into()),
        url: secrecy::SecretString::new("https://example.com".to_string().into()),
        password: secrecy::SecretString::new("secret".into()),
        notes: None,
        score: None,
        created_at: None,
    };
    create_stored_data_pipeline_bulk(&pool, user_id, vec![stored_raw_password])
        .await
        .expect("Failed to encrypt");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch");

    // Crea un altro utente con password diversa — il salt derivato sarà diverso
    let other_user_id = create_test_user(&pool, "other_user", "DifferentPassword456!").await;
    let wrong_auth = fetch_user_auth_from_id(&pool, other_user_id).await.unwrap();

    // Tentativo di decifrare con la chiave sbagliata
    let result = decrypt_bulk_stored_data(wrong_auth, stored_passwords, None).await;
    assert!(result.is_err(), "Should fail with wrong decryption key");
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
    let mut stored_raw_passwords: Vec<StoredRawPassword> = vec![];
    for (url, raw_pwd) in &passwords {
        let stored_raw_password = StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            name: String::new(),
            username: SecretString::new(String::new().into()),
            url: SecretString::new(url.to_string().into()),
            password: SecretString::new(raw_pwd.to_string().into()),
            notes: None,
            score: None,
            created_at: None,
        };
        stored_raw_passwords.push(stored_raw_password);
    }
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to encrypt password");
    // Recupera tutte le password
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(stored_passwords.len(), 3, "Should have 3 stored passwords");

    // Verifica che ogni password possa essere decifrata correttamente usando decrypt_bulk_stored_data
    let user_auth = fetch_user_auth_from_id(&pool, user_id).await.unwrap();
    let decrypted = decrypt_bulk_stored_data(user_auth, stored_passwords, None)
        .await
        .expect("Failed to decrypt passwords");
    for (i, (_expected_url, expected_password)) in passwords.iter().enumerate() {
        assert_eq!(decrypted[i].password.expose_secret(), *expected_password);
    }
}

#[tokio::test]
async fn test_multiple_passwords_for_same_user_with_predefined_strength() {
    // Initialize blacklist for accurate password evaluation
    let _ = pwd_strength::init_blacklist();

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
    let mut stored_raw_passwords: Vec<StoredRawPassword> = vec![];
    for (url, raw_pwd, strength) in &passwords {
        let stored_raw_password = StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            name: String::new(),
            username: SecretString::new(String::new().into()),
            url: SecretString::new(url.to_string().into()),
            password: SecretString::new(raw_pwd.to_string().into()),
            notes: Some(SecretString::new("Ciao io sono una nota".into())),
            score: strength.score,
            created_at: None,
        };
        stored_raw_passwords.push(stored_raw_password);
    }
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to encrypt password");

    // Recupera tutte le password
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    assert_eq!(stored_passwords.len(), 3, "Should have 3 stored passwords");

    // Verifica che ogni password possa essere decifrata correttamente usando decrypt_bulk_stored_data
    let user_auth = fetch_user_auth_from_id(&pool, user_id).await.unwrap();
    let decrypted = decrypt_bulk_stored_data(user_auth, stored_passwords, None)
        .await
        .expect("Failed to decrypt passwords");
    for (i, (_expected_url, expected_password, expected_strength)) in passwords.iter().enumerate() {
        let expected_strength_score = expected_strength.score.map(|s| s.value()).unwrap_or(0);
        assert_eq!(decrypted[i].score.map(|s| s.value()), Some(expected_strength_score));
        assert_eq!(decrypted[i].password.expose_secret(), *expected_password);
    }
}

#[tokio::test]
async fn test_encrypted_password_is_different_from_original() {
    let pool = setup_test_db().await;

    let user_id = create_test_user(&pool, "testuser4", "MasterPass789!").await;

    let raw_password = "MyPassword123";
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new("https://encrypted.com".to_string().into()),
        password: SecretString::new(raw_password.into()),
        notes: None,
        score: None,
        created_at: None,
    };
    let stored_raw_passwords = vec![stored_raw_password];
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
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
    let user_auth = fetch_user_auth_from_id(&pool, user_id).await.unwrap();
    let decrypted = decrypt_bulk_stored_data(user_auth, stored_passwords, None)
        .await
        .expect("Failed to decrypt password");
    assert_eq!(decrypted[0].password.expose_secret(), raw_password);
}

#[tokio::test]
async fn test_url_and_notes_are_encrypted() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_enc", "MasterPass123!").await;

    let raw_password = SecretString::new("MySecurePassword456".into());
    let url = "https://secret-url.com".to_string();
    let notes = Some("Confidential notes".to_string());
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new(url.clone().into()),
        password: raw_password.clone(),
        notes: Some(SecretString::new(notes.clone().unwrap().into())),
        score: None,
        created_at: None,
    };
    let stored_raw_passwords = vec![stored_raw_password];
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to encrypt data");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    let stored = &stored_passwords[0];

    // Verify url is NOT plaintext
    let url_bytes: &[u8] = stored.url.expose_secret().as_ref();
    let url_plain: &[u8] = url.as_bytes();
    assert_ne!(url_bytes, url_plain, "url should be encrypted");

    // Verify notes are NOT plaintext
    if let Some(notes_enc) = &stored.notes {
        let notes_bytes: &[u8] = notes_enc.expose_secret().as_ref();
        let notes_plain: &[u8] = notes.as_ref().unwrap().as_bytes();
        assert_ne!(notes_bytes, notes_plain, "Notes should be encrypted");
    }

    // Verify nonces are 12 bytes
    assert_eq!(stored.url_nonce.len(), 12);
    assert_eq!(stored.password_nonce.len(), 12);
    if let Some(nn) = &stored.notes_nonce {
        assert_eq!(nn.len(), 12);
    }
}

#[tokio::test]
async fn test_decrypt_url_and_notes_roundtrip() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_rt", "MasterPass123!").await;

    let raw_password = SecretString::new("MyPassword".into());
    let url = "MySecretService".to_string();
    let notes = Some(SecretString::new("My secret notes".into()));
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new(url.clone().into()),
        password: raw_password.clone(),
        notes: notes.clone(),
        score: None,
        created_at: None,
    };
    let stored_raw_passwords = vec![stored_raw_password];
    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to encrypt data");

    // Usa get_stored_raw_passwords che recupera correttamente l'UserAuth dal DB
    let decrypted = crate::backend::password_utils::get_stored_raw_passwords(&pool, user_id)
        .await
        .expect("Failed to decrypt");

    assert_eq!(decrypted.len(), 1);
    assert_eq!(decrypted[0].url.expose_secret(), url);
    // Confronta notes: Option<SecretString> vs Option<String>
    match (&decrypted[0].notes, &notes) {
        (Some(dec_notes), Some(exp_notes)) => {
            assert_eq!(dec_notes.expose_secret(), exp_notes.expose_secret());
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

// ============ Test per stored_passwords_migration_pipeline_with_progress ============
// Flusso corretto:
// 1. Creare utente → password viene hashata
// 2. Salvare StoredPassword (criptate con la vecchia master)
// 3. Cambiare password → vecchio HASH salvato in temp_old_password
// 4. Recuperare temp_old_password (è già un hash Argon2)
// 5. Passare quell'hash a stored_passwords_migration_pipeline_with_progress

#[tokio::test]
async fn test_password_migration_single_password() {
    let pool = setup_test_db().await;
    let old_password = "OldMasterPass123!";
    let new_password = "NewMasterPass456!";

    // 1. Crea utente con vecchia password
    let user_id = create_test_user(&pool, "migration_single", old_password).await;

    // 2. Crea StoredPassword da migrare (criptata con vecchia master)
    let raw_password = SecretString::new("MySecurePassword789".into());
    let url = "https://example.com".to_string();
    let notes = Some(SecretString::new("Test notes".into()));
    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: String::new(),
        username: SecretString::new(String::new().into()),
        url: SecretString::new(url.clone().into()),
        password: raw_password.clone(),
        notes: notes.clone(),
        score: None,
        created_at: None,
    };
    create_stored_data_pipeline_bulk(&pool, user_id, vec![stored_raw_password])
        .await
        .expect("Failed to save initial password");

    // 3. Cambia master password → salva VECCHIO HASH in temp_old_password
    save_or_update_user(
        &pool,
        Some(user_id),
        format!("migration_single_{}", user_id),
        Some(SecretString::new(new_password.into())),
        None,
    )
    .await
    .expect("Failed to update password");

    // 4. Recupera temp_old_password (HASH della vecchia password)
    let temp_old_hash = fetch_user_temp_old_password(&pool, user_id)
        .await
        .expect("Failed to fetch temp_old_password")
        .expect("temp_old_password should exist");

    // 5. Esegui migration passando l'HASH
    let result = stored_passwords_migration_pipeline_with_progress(&pool, user_id, temp_old_hash, None).await;
    assert!(result.is_ok(), "Migration should succeed: {:?}", result);

    // 6. Verifica temp_old_password rimosso
    let temp_old_after = fetch_user_temp_old_password(&pool, user_id)
        .await
        .expect("Failed to fetch temp_old_password");
    assert!(
        temp_old_after.is_none(),
        "temp_old_password should be removed, but was: {:?}",
        temp_old_after
    );

    // 7. Verifica decriptazione con NUOVA master
    let decrypted = get_stored_raw_passwords(&pool, user_id)
        .await
        .expect("Failed to decrypt with new password");
    assert_eq!(decrypted.len(), 1);
    assert_eq!(
        decrypted[0].password.expose_secret(),
        raw_password.expose_secret()
    );
    assert_eq!(decrypted[0].url.expose_secret(), url);
}

#[tokio::test]
async fn test_password_migration_multiple_passwords() {
    let pool = setup_test_db().await;
    let old_password = "OldMasterPass123!";
    let new_password = "NewMasterPass456!";

    // 1. Crea utente
    let user_id = create_test_user(&pool, "migration_multi", old_password).await;

    // 2. Crea multiple StoredPassword
    let passwords_data = vec![
        ("https://site1.com", "Password1", Some("Note 1")),
        ("https://site2.com", "Password2", None),
        ("https://site3.com", "Password3", Some("Note 3")),
    ];

    let stored_raw_passwords: Vec<StoredRawPassword> = passwords_data
        .iter()
        .map(|(url, pwd, note)| StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            name: String::new(),
            username: SecretString::new(String::new().into()),
            url: SecretString::new(url.to_string().into()),
            password: SecretString::new(pwd.to_string().into()),
            notes: note.map(|n| SecretString::new(n.into())),
            score: None,
            created_at: None,
        })
        .collect();

    create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords)
        .await
        .expect("Failed to save initial passwords");

    // 3. Cambia master password
    save_or_update_user(
        &pool,
        Some(user_id),
        format!("migration_multi_{}", user_id),
        Some(SecretString::new(new_password.into())),
        None,
    )
    .await
    .expect("Failed to update password");

    // 4. Recupera temp_old_password e esegui migration
    let temp_old_hash = fetch_user_temp_old_password(&pool, user_id)
        .await
        .expect("Failed to fetch temp_old_password")
        .expect("temp_old_password should exist");

    let result = stored_passwords_migration_pipeline_with_progress(&pool, user_id, temp_old_hash, None).await;
    assert!(result.is_ok(), "Migration should succeed: {:?}", result);

    // 5. Verifica tutte le password
    let decrypted = get_stored_raw_passwords(&pool, user_id)
        .await
        .expect("Failed to decrypt");
    assert_eq!(decrypted.len(), passwords_data.len());

    for (i, (url, pwd, note)) in passwords_data.iter().enumerate() {
        assert_eq!(decrypted[i].url.expose_secret(), *url);
        assert_eq!(decrypted[i].password.expose_secret(), *pwd);
        match (note, &decrypted[i].notes) {
            (Some(exp), Some(act)) => assert_eq!(act.expose_secret(), *exp),
            (None, None) => {}
            _ => panic!("Notes mismatch at index {}", i),
        }
    }
}

#[tokio::test]
async fn test_password_migration_empty_passwords() {
    let pool = setup_test_db().await;
    let old_password = "OldMasterPass123!";
    let new_password = "NewMasterPass456!";

    // 1. Crea utente (NESSUNA StoredPassword)
    let user_id = create_test_user(&pool, "migration_empty", old_password).await;

    // 2. Cambia master password
    save_or_update_user(
        &pool,
        Some(user_id),
        format!("migration_empty_{}", user_id),
        Some(SecretString::new(new_password.into())),
        None,
    )
    .await
    .expect("Failed to update password");

    // 3. Recupera temp_old_password e esegui migration
    let temp_old_hash = fetch_user_temp_old_password(&pool, user_id)
        .await
        .expect("Failed to fetch temp_old_password")
        .expect("temp_old_password should exist");

    let result = stored_passwords_migration_pipeline_with_progress(&pool, user_id, temp_old_hash, None).await;
    assert!(
        result.is_ok(),
        "Migration with no passwords should succeed: {:?}",
        result
    );

    // 4. Verifica temp_old_password rimosso
    let temp_old_after = fetch_user_temp_old_password(&pool, user_id)
        .await
        .expect("Failed to fetch temp_old_password");
    assert!(
        temp_old_after.is_none(),
        "temp_old_password should be removed, but was: {:?}",
        temp_old_after
    );
}

// ============ Test per ExcludedSymbolSet ============

/// Test che verifica che le password generate rispettino ExcludedSymbolSet.
///
/// Usa tre set di simboli esclusi:
/// - Piccolo: 3 simboli comuni (!@#)
/// - Medio: 10 simboli (!@#$%^&*())
/// - Massimale: tutti i simboli speciali della tastiera US standard
///
/// Per ogni set, genera 10 password e verifica che NESSUNA contenga i simboli esclusi.
#[tokio::test]
async fn test_excluded_symbol_set_respected() {
    // Definisci i tre set di simboli esclusi
    let small_excluded: String = "!@#".to_string();
    let medium_excluded: String = "!@#$%^&*()".to_string();
    // Massimale: tutti i simboli speciali della tastiera US (senza underscore e trattino che sono comuni)
    let maximal_excluded: String = "!@#$%^&*()=+[]{}|;:'\",.<>/?~`".to_string();

    let test_cases: Vec<(&str, String)> = vec![
        ("small", small_excluded),
        ("medium", medium_excluded),
        ("maximal", maximal_excluded),
    ];

    for (name, excluded_str) in test_cases {
        let excluded_set = ExcludedSymbolSet::from(excluded_str.clone());

        let config = PasswordGeneratorConfig {
            id: Some(1),
            settings_id: 1,
            length: 20,
            symbols: 3,
            numbers: true,
            uppercase: true,
            lowercase: true,
            excluded_symbols: excluded_set.clone(),
        };

        // Genera 10 password e verifica che nessuna contenga i simboli esclusi
        for i in 0..10 {
            let password = generate_suggested_password(Some(config.clone()));
            let pwd_str = password.expose_secret();

            // Verifica che NESSUN simbolo escluso sia presente
            for excluded_char in excluded_set.iter() {
                assert!(
                    !pwd_str.contains(*excluded_char),
                    "[{}] Password #{} contains excluded symbol '{}': {}",
                    name,
                    i + 1,
                    excluded_char,
                    pwd_str
                );
            }

            // Verifica anche che la password abbia ancora simboli (non solo alfanumerici)
            let symbol_count = pwd_str.chars().filter(|c| !c.is_alphanumeric()).count();
            assert!(
                symbol_count >= config.symbols as usize,
                "[{}] Password #{} should have at least {} symbols, got {}: {}",
                name,
                i + 1,
                config.symbols,
                symbol_count,
                pwd_str
            );
        }

        println!(
            "[{}] All 10 passwords respected excluded symbols: {}",
            name, excluded_str
        );
    }
}

/// Test che verifica il caso limite: set di simboli esclusi vuoto.
#[tokio::test]
async fn test_excluded_symbol_set_empty() {
    let excluded_set = ExcludedSymbolSet::default(); // Set vuoto

    let config = PasswordGeneratorConfig {
        id: Some(1),
        settings_id: 1,
        length: 16,
        symbols: 2,
        numbers: true,
        uppercase: true,
        lowercase: true,
        excluded_symbols: excluded_set,
    };

    // Genera password - dovrebbe funzionare senza problemi
    let password = generate_suggested_password(Some(config));
    let pwd_str = password.expose_secret();

    // Verifica che abbia simboli (nessuno escluso)
    let symbol_count = pwd_str.chars().filter(|c| !c.is_alphanumeric()).count();
    assert!(
        symbol_count >= 2,
        "Password should have at least 2 symbols, got {}: {}",
        symbol_count,
        pwd_str
    );

    println!("Empty excluded set test passed: {}", pwd_str);
}

#[cfg(test)]
mod diceware_tests {
    use super::*;
    use crate::backend::password_utils::{generate_diceware_password, DicewareGenConfig};
    use diceware::EmbeddedList;

    #[test]
    fn test_generate_diceware_default_config() {
        let config = DicewareGenConfig {
            word_count: 6,
            special_chars: 0,
            force_special_chars: false,
            numbers: 0,
            language: EmbeddedList::EN,
        };
        let pwd = generate_diceware_password(config);
        let pwd_str = pwd.expose_secret();
        // CamelCase: no spaces, each word starts uppercase
        assert!(!pwd_str.contains(' '), "Should be CamelCase (no spaces)");
        // 6 words = at least 6 characters
        assert!(pwd_str.len() >= 6, "Should have at least 6 characters");
        // No special chars (special_chars == 0)
        assert!(
            !pwd_str.chars().any(|c| !c.is_alphanumeric()),
            "Should have no special characters when special_chars == 0"
        );
    }

    #[test]
    fn test_generate_diceware_with_special_chars() {
        let config = DicewareGenConfig {
            word_count: 6,
            special_chars: 1,
            force_special_chars: false,
            numbers: 0,
            language: EmbeddedList::EN,
        };
        let pwd = generate_diceware_password(config);
        let pwd_str = pwd.expose_secret();
        let special_count = pwd_str.chars().filter(|c| !c.is_alphanumeric()).count();
        assert!(special_count >= 1, "Should have at least 1 special character, got {special_count}");
    }

    #[test]
    fn test_generate_diceware_italian() {
        let config = DicewareGenConfig {
            word_count: 4,
            special_chars: 0,
            force_special_chars: false,
            numbers: 0,
            language: EmbeddedList::IT,
        };
        let pwd = generate_diceware_password(config);
        let pwd_str = pwd.expose_secret();
        assert!(pwd_str.len() >= 4);
        assert!(!pwd_str.contains(' '));
    }

    #[test]
    fn test_detect_system_language_returns_valid() {
        // Just verify it returns a valid EmbeddedList variant without panicking
        let _lang = crate::backend::password_utils::detect_system_language();
    }
}

#[tokio::test]
async fn test_diceware_registration_default_settings() {
    let pool = setup_test_db().await;

    // Use register_user_with_settings — it creates user + user_settings + passwords_generation_settings + diceware_generation_settings
    let user_id = crate::backend::db_backend::register_user_with_settings(
        &pool,
        format!("diceware_reg_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
        Some(secrecy::SecretString::new("TestPass123!".into())),
        None,
        pwd_types::PasswordPreset::God,
    )
    .await
    .expect("Should register user with settings");

    // Fetch diceware settings for the newly registered user
    let settings = crate::backend::db_backend::fetch_diceware_settings(&pool, user_id)
        .await
        .expect("Should fetch diceware settings for new user");

    // Verify defaults
    assert_eq!(settings.word_count, 6);
    assert_eq!(settings.special_chars, 0);
    assert!(!settings.force_special_chars);
    assert_eq!(settings.numbers, 0);
    // Language depends on system locale — just verify it's one of the valid variants
    assert!(matches!(
        settings.language,
        crate::backend::settings_types::DicewareLanguage::EN
            | crate::backend::settings_types::DicewareLanguage::IT
            | crate::backend::settings_types::DicewareLanguage::FR
    ));
}
