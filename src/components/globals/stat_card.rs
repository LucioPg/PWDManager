// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;

/// Variante di colore per la statistica
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum StatVariant {
    #[default]
    Primary,
    #[allow(dead_code)]
    Success,
    #[allow(dead_code)]
    Warning,
    #[allow(dead_code)]
    Info,
    #[allow(dead_code)]
    Error,
}

#[allow(dead_code)]
impl StatVariant {
    pub fn as_css_class(&self) -> &'static str {
        match self {
            StatVariant::Primary => "stat-value-primary",
            StatVariant::Success => "stat-value-success",
            StatVariant::Warning => "stat-value-warning",
            StatVariant::Info => "stat-value-info",
            StatVariant::Error => "stat-value-error",
        }
    }
}

/// Componente StatCard - Card per visualizzare una statistica
///
/// # Esempio
/// ```rust
/// rsx! {
///     StatCard {
///         value: "42".to_string(),
///         label: "Total Passwords".to_string(),
///         variant: StatVariant::Primary,
///     }
/// }
/// ```
#[component]
pub fn StatCard(
    /// Valore numerico da visualizzare
    value: String,
    /// Etichetta descrittiva della statistica
    label: String,
    /// Variante di colore (default: Primary)
    #[props(default)]
    variant: StatVariant,
    /// on click handler
    on_click: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "stat-card", onclick: move |evt| on_click.call(evt),
            p { class: "stat-value {variant.as_css_class()}", "{value}" }
            p { class: "stat-label", "{label}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_variant_css_class() {
        assert_eq!(StatVariant::Primary.as_css_class(), "stat-value-primary");
        assert_eq!(StatVariant::Success.as_css_class(), "stat-value-success");
        assert_eq!(StatVariant::Warning.as_css_class(), "stat-value-warning");
        assert_eq!(StatVariant::Info.as_css_class(), "stat-value-info");
        assert_eq!(StatVariant::Error.as_css_class(), "stat-value-error");
    }

    #[test]
    fn test_stat_variant_default() {
        let default_variant = StatVariant::default();
        assert_eq!(default_variant, StatVariant::Primary);
    }
}
