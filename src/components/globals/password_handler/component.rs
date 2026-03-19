// src/components/globals/password_handler/component.rs
//
// WRAPPER: Fornisce callback DB a pwd-dioxus::PasswordHandler
// Questo file mantiene backward compatibility con il resto del progetto

use crate::auth::AuthState;
use crate::backend::db_backend::{fetch_diceware_settings, fetch_user_passwords_generation_settings};
use crate::backend::password_utils::{generate_diceware_password, generate_suggested_password, DicewareGenConfig};
use crate::backend::evaluate_password_strength_tx;
use dioxus::prelude::*;
use pwd_dioxus::{PasswordHandler as LibPasswordHandler, FormSecret, EvaluationResult};
use pwd_dioxus::password::GenerationMethod;
use pwd_types::{PasswordChangeResult, PasswordScore};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    /// Callback quando la password cambia, include password, score, strength e reasons
    pub on_password_change: Callback<PasswordChangeResult>,
    #[props(default = true)]
    pub password_required: bool,
    pub initial_password: Option<FormSecret>,
    pub initial_score: Option<PasswordScore>,
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();

    // State per generazione password
    let mut generated_pwd = use_signal(|| None::<FormSecret>);
    let mut is_generating = use_signal(|| false);

    // Callback per valutazione password (chiama DB)
    let pool_for_eval = pool.clone();
    let on_evaluate = use_callback(move |(form_secret, token, tx): (FormSecret, Arc<CancellationToken>, mpsc::Sender<EvaluationResult>)| {
        let pool = pool_for_eval.clone();

        spawn(async move {
            // Chiama la funzione di valutazione dal backend
            let (eval_tx, mut eval_rx) = mpsc::channel(1);
            evaluate_password_strength_tx(&form_secret.0, (*token).clone(), eval_tx).await;

            if let Some(eval) = eval_rx.recv().await {
                let result = EvaluationResult {
                    score: eval.score,
                    strength: eval.strength(),
                    reasons: eval.reasons,
                };
                let _ = tx.send(result).await;
            }
        });
    });

    // Callback per generazione password (chiama DB)
    let auth_for_gen = auth_state.clone();
    let pool_for_gen = pool.clone();
    let on_suggest_method = use_callback(move |method: GenerationMethod| {
        let pool = pool_for_gen.clone();
        let auth = auth_for_gen.clone();
        let mut is_gen = is_generating.clone();
        let mut gen_pwd = generated_pwd.clone();

        spawn(async move {
            is_gen.set(true);

            match method {
                GenerationMethod::Random => {
                    let config = if let Some(user) = auth.get_user() {
                        fetch_user_passwords_generation_settings(&pool, user.id).await.ok()
                    } else {
                        None
                    };
                    let pwd = generate_suggested_password(config);
                    gen_pwd.set(Some(FormSecret(pwd)));
                }
                GenerationMethod::Diceware => {
                    let pwd = if let Some(user) = auth.get_user() {
                        fetch_diceware_settings(&pool, user.id)
                            .await
                            .ok()
                            .map(DicewareGenConfig::from)
                    } else {
                        None
                    };
                    let config = pwd.unwrap_or_else(|| DicewareGenConfig {
                        word_count: 6,
                        special_chars: 0,
                        force_special_chars: false,
                        numbers: 0,
                        language: crate::backend::password_utils::detect_system_language().into(),
                    });
                    let generated = generate_diceware_password(config);
                    gen_pwd.set(Some(FormSecret(generated)));
                }
            }

            is_gen.set(false);
        });
    });

    // Callback per cambiamento password - passa l'intero risultato al consumer
    let on_change = use_callback(move |result: PasswordChangeResult| {
        props.on_password_change.call(result);
    });

    rsx! {
        LibPasswordHandler {
            on_password_change: on_change,
            password_required: props.password_required,
            initial_password: props.initial_password,
            initial_score: props.initial_score,
            on_suggest_method: Some(on_suggest_method),
            generated_password: Some(generated_pwd),
            is_generating: Some(is_generating),
            on_evaluate: Some(on_evaluate),
            password_label: "Password".to_string(),
            show_strength_bar: true,
            show_suggest_button: true,
        }
    }
}
