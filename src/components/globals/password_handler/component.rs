use crate::backend::password_types_helper::PasswordStrength;
use crate::backend::strength_utils::evaluate_password_strength_tx;
use crate::components::globals::form_field::{FormSecret, InputType};
use dioxus::core::Task;
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
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
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    // Internal state
    let mut password = use_signal(|| FormSecret(SecretString::new(String::new().into())));
    let mut repassword = use_signal(|| FormSecret(SecretString::new(String::new().into())));
    let mut strength = use_signal(|| PasswordStrength::NotEvaluated);
    let mut reasons = use_signal(|| Vec::<String>::new());
    let mut is_evaluating = use_signal(|| false);
    let mut score = use_signal(|| Option::<i32>::None);

    let mut debounce_task = use_signal(|| None::<Task>);
    let mut cancel_token = use_signal(|| Arc::new(CancellationToken::new()));

    // Callback triggered when password changes
    let on_password_change = move |new_pwd: FormSecret| {
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
            // Start debounce timer
            let mut strength_sig = strength.clone();
            let mut reasons_sig = reasons.clone();
            let mut evaluating_sig = is_evaluating.clone();
            let mut score_sig = score.clone();
            let on_change = props.on_password_change.clone();

            let task = spawn(async move {
                sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                if token.is_cancelled() {
                    return;
                }

                evaluating_sig.set(true);

                let (tx, mut rx) = mpsc::channel(1);
                evaluate_password_strength_tx(&new_pwd.0, (*token).clone(), tx).await;

                if let Some(eval) = rx.recv().await {
                    strength_sig.set(eval.strength);
                    reasons_sig.set(eval.reasons);
                    score_sig.set(eval.score);
                    on_change.call(new_pwd);
                }

                evaluating_sig.set(false);
            });

            debounce_task.set(Some(task));
        }
    };

    // Callback triggered when repassword changes
    let on_repassword_change = move |new_re_pwd: FormSecret| {
        repassword.set(new_re_pwd.clone());

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
        let pwd = password.read().clone();
        let pwd_match = pwd.0.expose_secret() == new_re_pwd.0.expose_secret();
        let is_empty = pwd.0.expose_secret().is_empty();

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
                    return;
                }

                evaluating_sig.set(true);

                let (tx, mut rx) = mpsc::channel(1);
                evaluate_password_strength_tx(&pwd.0, (*token).clone(), tx).await;

                if let Some(eval) = rx.recv().await {
                    strength_sig.set(eval.strength);
                    reasons_sig.set(eval.reasons);
                    score_sig.set(eval.score);
                    on_change.call(pwd);
                }

                evaluating_sig.set(false);
            });

            debounce_task.set(Some(task));
        }
    };

    // Cleanup on component unmount
    use_drop(move || {
        if let Some(task) = debounce_task.read().as_ref() {
            task.cancel();
        }
        cancel_token.read().cancel();
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
