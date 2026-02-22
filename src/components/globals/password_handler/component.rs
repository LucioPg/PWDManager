use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_passwords_generation_settings;
use crate::backend::password_types_helper::{PasswordScore, PasswordStrength};
use crate::backend::password_utils::generate_suggested_password;
use crate::backend::strength_utils::evaluate_password_strength_tx;
use crate::components::globals::form_field::{FormSecret, InputType};
use crate::components::globals::svgs::MagicWandIcon;
use dioxus::core::Task;
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

const DEBOUNCE_MS: u64 = 500;

#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    pub on_password_change: Callback<FormSecret>,
    #[props(default = true)]
    pub password_required: bool,
    /// Password iniziale per modalità edit (pre-compilata)
    #[props(default = None)]
    pub initial_password: Option<FormSecret>,
    /// Strength pre-calcolata per modalità edit
    #[props(default = None)]
    pub initial_score: Option<PasswordScore>,
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    // Context per AuthState e SqlitePool
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();

    // Internal state - inizializza con valori iniziali se presenti (modalità edit)
    let initial_pwd = props
        .initial_password
        .clone()
        .unwrap_or_else(|| FormSecret(SecretString::new(String::new().into())));
    let mut password = use_signal(|| initial_pwd.clone());
    #[allow(unused_mut)]
    let mut repassword = use_signal(|| initial_pwd.clone());
    #[allow(unused_mut)]
    let mut score = use_signal(|| props.initial_score.clone());
    let mut strength = use_memo(move || {
        // 1. Leggiamo il segnale (restituisce Option<PasswordScore>)
        let score_opt = score.read().clone();

        // 2. Estraiamo il valore Option<i64> interno (se esiste)
        let raw_val = score_opt.map(|s| s.value() as i64);

        // 3. Passiamo l'Option<i64> alla tua funzione
        PasswordScore::get_strength(raw_val)
    });
    let mut reasons = use_signal(|| Vec::<String>::new());
    #[allow(unused_mut)]
    let mut is_evaluating = use_signal(|| false);

    let mut debounce_task = use_signal(|| None::<Task>);
    let mut cancel_token = use_signal(|| Arc::new(CancellationToken::new()));

    // Callback triggered when password changes
    let mut on_password_change = move |new_pwd: FormSecret| {
        password.set(new_pwd.clone());

        // Reset evaluation state
        strength.set(PasswordStrength::NotEvaluated);
        reasons.set(Vec::new());
        score.set(None);

        // Cancel previous task
        if let Some(task) = debounce_task.read().as_ref() {
            task.cancel();
        }
        debounce_task.set(None);

        // Create new cancellation token
        let token = Arc::new(CancellationToken::new());
        cancel_token.set(token.clone());

        // Check if passwords match and are not empty
        let re_pwd = repassword.read().clone();
        let pwd_match = new_pwd.0.expose_secret() == re_pwd.0.expose_secret();
        let is_empty = new_pwd.0.expose_secret().is_empty();

        tracing::debug!(
            "PasswordHandler - Password changed: empty={}, match={}",
            is_empty,
            pwd_match
        );

        if !is_empty && pwd_match {
            // Start debounce timer
            let mut strength_sig = strength.clone();
            let mut reasons_sig = reasons.clone();
            let mut evaluating_sig = is_evaluating.clone();
            let mut score_sig = score.clone();
            let on_change = props.on_password_change.clone();

            let task = spawn(async move {
                sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                if token.is_cancelled() {
                    tracing::debug!("PasswordHandler - Task cancelled after debounce");
                    return;
                }

                evaluating_sig.set(true);

                let (tx, mut rx) = mpsc::channel(1);
                evaluate_password_strength_tx(&new_pwd.0, (*token).clone(), tx).await;

                if let Some(eval) = rx.recv().await {
                    tracing::debug!(
                        "PasswordHandler - Evaluation result: score={:?}, strength={:?}, reasons={:?}",
                        eval.score,
                        eval.strength(),
                        eval.reasons
                    );
                    strength_sig.set(eval.strength());
                    reasons_sig.set(eval.reasons);
                    score_sig.set(eval.score);
                    on_change.call(new_pwd);
                } else {
                    tracing::debug!("PasswordHandler - No evaluation received from channel");
                }

                evaluating_sig.set(false);
            });

            debounce_task.set(Some(task));
        }
    };

    // Callback triggered when repassword changes (only updates repassword, triggers evaluation if passwords match)
    let on_repassword_change = move |new_pwd: FormSecret| {
        repassword.set(new_pwd.clone());

        // Check if passwords now match - if so, trigger evaluation
        let pwd = password.read().clone();
        let pwd_match = pwd.0.expose_secret() == new_pwd.0.expose_secret();
        let is_empty = pwd.0.expose_secret().is_empty();

        tracing::debug!(
            "PasswordHandler - Repassword changed: empty={}, match={}",
            is_empty,
            pwd_match
        );

        if !is_empty && pwd_match {
            // Passwords match, trigger evaluation by calling the main callback
            on_password_change(pwd);
        }
    };

    // Cleanup on component unmount
    use_drop(move || {
        if let Some(task) = debounce_task.read().as_ref() {
            task.cancel();
        }
        cancel_token.read().cancel();
    });

    // Callback per generare password suggerita
    let auth_state_for_callback = auth_state.clone();
    let on_suggest_password = use_callback(move |_| {
        // Cloni dei signal per l'async block
        let mut password_sig = password.clone();
        let mut repassword_sig = repassword.clone();
        let on_change = props.on_password_change.clone();
        let pool = pool.clone();
        let auth_state_inner = auth_state_for_callback.clone();

        spawn(async move {
            // Se l'utente è loggato, fetcha il config dal DB, altrimenti usa None (default God)
            let config = if let Some(user) = auth_state_inner.get_user() {
                fetch_user_passwords_generation_settings(&pool, user.id).await.ok()
            } else {
                None
            };

            // Genera la password suggerita (usa default God se config è None)
            let suggested = generate_suggested_password(config);

            // Aggiorna entrambi i campi
            let form_secret = FormSecret(suggested);
            password_sig.set(form_secret.clone());
            repassword_sig.set(form_secret.clone());

            // Notifica il parent della nuova password
            on_change.call(form_secret);

            tracing::debug!("PasswordHandler - Suggested password generated successfully");
        });
    });

    rsx! {
        div { class: "password-handler flex flex-col gap-3",
            // Password field
            crate::components::globals::form_field::FormField::<FormSecret> {
                label: "Password".to_string(),
                input_type: InputType::Password,
                placeholder: "Enter your password".to_string(),
                value: password,
                required: props.password_required,
                autocomplete: false,
                on_change: on_password_change,
                show_visibility_toggle: true,
                forbid_spaces: true,
            }

            // Retype password field
            crate::components::globals::form_field::FormField::<FormSecret> {
                label: "Confirm Password".to_string(),
                input_type: InputType::Password,
                placeholder: "Confirm your password".to_string(),
                value: repassword,
                required: props.password_required,
                autocomplete: false,
                on_change: on_repassword_change,
                show_visibility_toggle: true,
                forbid_spaces: true,
            }

            // Pulsante suggerimento password
            div { class: "flex justify-end",
                button {
                    class: "btn btn-ghost btn-sm gap-2 tooltip",
                    "data-tip": "suggest password",
                    r#type: "button",
                    onclick: on_suggest_password,
                    MagicWandIcon {
                        size: "16".to_string(),
                        stroke_width: "2".to_string(),
                        class: None,
                    }
                    span { class: "text-xs", "Suggest" }
                }
            }

            // Strength analyzer
            super::StrengthAnalyzer {
                strength: strength.read().clone(),
                reasons: reasons.read().clone(),
                is_evaluating: is_evaluating(),
                score: score.read().clone(),
            }

            // Password mismatch warning
            if !password.read().0.expose_secret().is_empty()
                && !repassword.read().0.expose_secret().is_empty()
                && password.read().0.expose_secret() != repassword.read().0.expose_secret()
            {
                div { class: "text-error-600", "Passwords do not match" }
            }
        }
    }
}
