use dioxus::prelude::*;

/// Dimensioni disponibili per lo Spinner
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum SpinnerSize {
    #[default]
    Small, // spinner-sm
    Medium, // spinner-md
    Large,  // spinner-lg
    XLarge, // spinner-xl
}

impl SpinnerSize {
    /// Restituisce la classe CSS per la dimensione
    pub fn as_class(&self) -> &'static str {
        match self {
            SpinnerSize::Small => "spinner-sm",
            SpinnerSize::Medium => "spinner-md",
            SpinnerSize::Large => "spinner-lg",
            SpinnerSize::XLarge => "spinner-xl",
        }
    }
}

/// Componente Spinner - Indicatore di caricamento animato
///
/// Componente riutilizzabile per mostrare stati di caricamento.
/// Usa animazioni CSS native per performance ottimali.
///
/// # Esempio
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use crate::components::globals::spinner::{Spinner, SpinnerSize};
///
/// // Spinner piccolo verde (stato di successo)
/// Spinner {
///     size: SpinnerSize::Small,
///     color: "text-success".to_string(),
/// }
///
/// // Spinner grande con colore personalizzato
/// Spinner {
///     size: SpinnerSize::Large,
///     color: "text-primary-600".to_string(),
/// }
/// ```
#[component]
pub fn Spinner(
    /// Dimensione dello spinner
    #[props(default)]
    size: SpinnerSize,
    /// Classe CSS per il colore (es. "spinner-success", "spinner-error", "")
    /// Se vuoto, usa lo spinner base (blu)
    #[props(default)]
    color_class: String,
    #[props(default = 0.8)] duration: f32,
    /// Classe CSS aggiuntiva per il container
    #[props(default)]
    class: Option<String>,
    /// Mostra o nasconde lo sfondo semi-trasparente
    #[props(default)]
    with_background: bool,
) -> Element {
    let size_class = size.as_class();
    let color_variant = if color_class.is_empty() {
        "".to_string()
    } else {
        format!(" {}", color_class)
    };

    let background = if with_background {
        " bg-primary-color/5 backdrop-blur-sm"
    } else {
        ""
    };

    let container_classes = if let Some(custom_class) = class {
        format!(" flex items-center justify-center {background} {custom_class}")
    } else {
        format!(" flex items-center justify-center {background}")
    };
    let spinner_classes = format!("spinner {} {color_class}", size_class);
    rsx! {
        div { class: "{container_classes}",
            div {
                class: "{spinner_classes}",
                style: format!("animation-duration: {duration}s;"),
            }
        }
    }
}
