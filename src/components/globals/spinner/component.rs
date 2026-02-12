use dioxus::prelude::*;

/// Dimensioni disponibili per lo Spinner
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum SpinnerSize {
    #[default]
    Small,   // 16px
    Medium,  // 24px
    Large,   // 32px
    XLarge,  // 48px
}

impl SpinnerSize {
    /// Restituisce le classi Tailwind per la dimensione
    pub fn as_classes(&self) -> &'static str {
        match self {
            SpinnerSize::Small => "w-4 h-4 border-2",        // 16px
            SpinnerSize::Medium => "w-6 h-6 border-2",       // 24px
            SpinnerSize::Large => "w-8 h-8 border-3",        // 32px
            SpinnerSize::XLarge => "w-12 h-12 border-4",     // 48px
        }
    }

    /// Restituisce la dimensione in pixel (per fallback CSS inline se necessario)
    pub fn as_pixels(&self) -> u32 {
        match self {
            SpinnerSize::Small => 16,
            SpinnerSize::Medium => 24,
            SpinnerSize::Large => 32,
            SpinnerSize::XLarge => 48,
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
    /// Classe colore Tailwind (es. "text-success", "text-primary-600")
    /// Il colore viene applicato al bordo visibile dello spinner
    #[props(default)]
    color: String,
    /// Classe CSS aggiuntiva per il container
    #[props(default)]
    class: Option<String>,
    /// Mostra o nasconde lo sfondo semi-trasparente
    #[props(default)]
    with_background: bool,
) -> Element {
    let size_classes = size.as_classes();
    let border_color = if color.is_empty() {
        "border-current".to_string()
    } else {
        format!("border-t-[{color}] border-r-transparent border-b-transparent border-l-transparent")
    };

    let background = if with_background {
        "bg-primary-color/5 backdrop-blur-sm"
    } else {
        ""
    };

    let container_classes = if let Some(custom_class) = class {
        format!("inline-flex items-center justify-center {} {}", background, custom_class)
    } else {
        format!("inline-flex items-center justify-center {}", background)
    };

    rsx! {
        div { class: "{container_classes}",
            div {
                class: "animate-spin rounded-full {size_classes} {border_color}",
                style: "animation-duration: 0.8s;",
            }
        }
    }
}
