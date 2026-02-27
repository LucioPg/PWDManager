use crate::form::{FormField, FormSecret, InputType};
use crate::icons::MagicWandIcon;
use super::StrengthAnalyzer;
use dioxus::prelude::*;
use dioxus::core::Task;
use pwd_types::{PasswordChangeResult, PasswordScore, PasswordStrength};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

const DEBOUNCE_MS: u64 = 500;

#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    /// Callback chiamato quando la password cambia con risultato completo
    pub on_password_change: Callback<PasswordChangeResult>,

    /// Se la password è richiesta
    #[props(default = true)]
    pub password_required: bool,

    /// Password iniziale per modalità edit (pre-compilata)
    pub initial_password: Option<FormSecret>,

    /// Strength pre-calcolata per modalità edit
    pub initial_score: Option<PasswordScore>,

    /// Callback chiamato quando utente clicca "Suggest" - il consumer deve generare la password
    #[props(default = None)]
    pub on_suggest: Option<Callback<()>>,

    /// Signal letta dal consumer per passare la password generata
    /// Il consumer imposta questo signal quando ha generato la password
    #[props(default = None)]
    pub generated_password: Option<Signal<Option<FormSecret>>>,

    /// Stato loading per generazione (opzionale, gestito dal consumer)
    #[props(default = None)]
    pub is_generating: Option<Signal<bool>>,

    /// Callback per valutare la password - restituisce (score, reasons) via channel
    /// Il consumer deve implementare la logica di valutazione
    pub on_evaluate: Option<Callback<(FormSecret, Arc<CancellationToken>, mpsc::Sender<EvaluationResult>)>>,

    /// Etichetta per il campo password
    #[props(default = "Password".to_string())]
    pub password_label: String,

    /// Mostra la barra della strength
    #[props(default = true)]
    pub show_strength_bar: bool,

    /// Mostra il pulsante suggest
    #[props(default = true)]
    pub show_suggest_button: bool,
}

/// Risultato della valutazione password (passato via channel)
#[derive(Clone)]
pub struct EvaluationResult {
    pub score: Option<PasswordScore>,
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    // Internal state - inizializza con valori iniziali se presenti (modalità edit)
    let initial_pwd = props
        .initial_password
        .clone()
        .unwrap_or_else(|| FormSecret(SecretString::new(String::new().into())));

    let mut password = use_signal(|| initial_pwd.clone());
    #[allow(unused_mut)]
    let mut repassword = use_signal(|| initial_pwd.clone());
    #[allow(unused_mut)]
    let mut score = use_signal(|| props.initial_score);
    let mut strength = use_memo(move || {
        let score_opt = score.read().clone();
        let raw_val = score_opt.map(|s| s.value() as i64);
        PasswordScore::get_strength(raw_val)
    });
    let mut reasons = use_signal(|| Vec::<String>::new());
    #[allow(unused_mut)]
    let mut is_evaluating = use_signal(|| false);

    let mut debounce_task = use_signal(|| None::<Task>);
    let mut cancel_token = use_signal(|| Arc::new(CancellationToken::new()));

    // Callback triggered when password changes
    let props_clone = props.clone();
    let mut on_password_change_internal = move |new_pwd: FormSecret| {
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

        if !is_empty && pwd_match {
            // Se c'è la callback di valutazione, usala
            if let Some(on_eval) = &props_clone.on_evaluate {
                let mut strength_sig = strength.clone();
                let mut reasons_sig = reasons.clone();
                let mut evaluating_sig = is_evaluating.clone();
                let mut score_sig = score.clone();
                let on_change = props_clone.on_password_change.clone();
                let on_eval = on_eval.clone();

                let task = spawn(async move {
                    sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                    if token.is_cancelled() {
                        return;
                    }

                    evaluating_sig.set(true);

                    let (tx, mut rx) = mpsc::channel(1);
                    on_eval.call((new_pwd.clone(), token.clone(), tx));

                    if let Some(eval) = rx.recv().await {
                        strength_sig.set(eval.strength);
                        let reasons_clone = eval.reasons.clone();
                        reasons_sig.set(reasons_clone.clone());
                        score_sig.set(eval.score);

                        on_change.call(PasswordChangeResult {
                            password: new_pwd.0.clone(),
                            score: eval.score,
                            strength: eval.strength,
                            reasons: reasons_clone,
                        });
                    }

                    evaluating_sig.set(false);
                });

                debounce_task.set(Some(task));
            } else {
                // Senza callback di valutazione, notifica solo il cambio password
                props_clone.on_password_change.call(PasswordChangeResult::new(new_pwd.0.clone()));
            }
        }
    };

    // Callback triggered when repassword changes
    let props_for_re = props.clone();
    let on_repassword_change = move |new_pwd: FormSecret| {
        repassword.set(new_pwd.clone());

        let pwd = password.read().clone();
        let pwd_match = pwd.0.expose_secret() == new_pwd.0.expose_secret();
        let is_empty = pwd.0.expose_secret().is_empty();

        if !is_empty && pwd_match {
            on_password_change_internal(pwd);
        }
    };

    // Cleanup on component unmount
    let debounce_task_clone = debounce_task.clone();
    let cancel_token_clone = cancel_token.clone();
    use_drop(move || {
        if let Some(task) = debounce_task_clone.read().as_ref() {
            task.cancel();
        }
        cancel_token_clone.read().cancel();
    });

    // Callback per suggerimento password - delega al consumer
    let suggest_onclick = {
        let on_suggest = props.on_suggest.clone();
        let is_gen = props.is_generating.clone();
        move |_| {
            if let Some(on_gen) = &on_suggest {
                on_gen.call(());
            }
        }
    };

    // Effect per sincronizzare generated_password
    let mut password_for_effect = password.clone();
    let mut repassword_for_effect = repassword.clone();
    let on_change_for_effect = props.on_password_change.clone();
    use_effect(move || {
        if let Some(gen_pwd_signal) = &props.generated_password {
            if let Some(new_pwd) = gen_pwd_signal.read().clone() {
                password_for_effect.set(new_pwd.clone());
                repassword_for_effect.set(new_pwd.clone());
                on_change_for_effect.call(PasswordChangeResult::new(new_pwd.0.clone()));
            }
        }
    });

    rsx! {
        div { class: "password-handler flex flex-col gap-3",
            // Password field
            FormField::<FormSecret> {
                label: props.password_label.clone(),
                input_type: InputType::Password,
                placeholder: "Enter your password".to_string(),
                value: password,
                required: props.password_required,
                autocomplete: false,
                on_change: on_password_change_internal,
                show_visibility_toggle: true,
                forbid_spaces: true,
            }

            // Retype password field
            FormField::<FormSecret> {
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

            // Pulsante suggerimento password (se abilitato)
            if props.show_suggest_button {
                div { class: "flex justify-end",
                    button {
                        class: "btn btn-ghost btn-sm gap-2 tooltip",
                        "data-tip": "suggest password",
                        r#type: "button",
                        onclick: suggest_onclick,
                        disabled: props.is_generating.map_or(false, |g| *g.read()),
                        MagicWandIcon {
                            size: "16".to_string(),
                            stroke_width: "2".to_string(),
                            class: None,
                        }
                        span { class: "text-xs", "Suggest" }
                    }
                }
            }

            // Strength analyzer
            StrengthAnalyzer {
                strength: strength.read().clone(),
                reasons: reasons.read().clone(),
                is_evaluating: is_evaluating(),
                score: score.read().clone(),
                show_bar: props.show_strength_bar,
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
