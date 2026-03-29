// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;
use pwd_types::PasswordStats;

/// Variante di colore per il badge delle statistiche nell'aside
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AsideBadgeVariant {
    Total,
    God,
    Epic,
    Strong,
    Medium,
    Weak,
}

impl AsideBadgeVariant {
    /// Restituisce la classe CSS per il badge
    pub fn badge_class(&self) -> &'static str {
        match self {
            AsideBadgeVariant::Total => "pwd-stats-aside__badge pwd-stats-aside__badge--total",
            AsideBadgeVariant::God => "pwd-stats-aside__badge pwd-stats-aside__badge--god",
            AsideBadgeVariant::Epic => "pwd-stats-aside__badge pwd-stats-aside__badge--epic",
            AsideBadgeVariant::Strong => "pwd-stats-aside__badge pwd-stats-aside__badge--strong",
            AsideBadgeVariant::Medium => "pwd-stats-aside__badge pwd-stats-aside__badge--medium",
            AsideBadgeVariant::Weak => "pwd-stats-aside__badge pwd-stats-aside__badge--weak",
        }
    }
}

/// Props per il componente StatsAside
#[component]
pub fn StatsAside(
    /// Statistiche delle password
    stats: PasswordStats,
    /// Callback quando si clicca su una statistica
    on_stat_click: EventHandler<Option<pwd_types::PasswordStrength>>,
    /// Filtro attualmente selezionato
    active_filter: Option<pwd_types::PasswordStrength>,
) -> Element {
    let mut is_expanded = use_signal(|| false);

    // Configurazione delle statistiche in ordine (dal più forte al più debole)
    let stat_items: [(char, usize, &str, AsideBadgeVariant, Option<pwd_types::PasswordStrength>); 6] = [
        ('T', stats.total, "Total", AsideBadgeVariant::Total, None),
        ('G', stats.god, "God", AsideBadgeVariant::God, Some(pwd_types::PasswordStrength::GOD)),
        ('E', stats.epic, "Epic", AsideBadgeVariant::Epic, Some(pwd_types::PasswordStrength::EPIC)),
        ('S', stats.strong, "Strong", AsideBadgeVariant::Strong, Some(pwd_types::PasswordStrength::STRONG)),
        ('M', stats.medium, "Medium", AsideBadgeVariant::Medium, Some(pwd_types::PasswordStrength::MEDIUM)),
        ('W', stats.weak, "Weak", AsideBadgeVariant::Weak, Some(pwd_types::PasswordStrength::WEAK)),
    ];

    rsx! {
        // Aside container con z-index alto per evitare shuttering
        aside {
            class: if is_expanded() { "pwd-stats-aside pwd-stats-aside--expanded" } else { "pwd-stats-aside pwd-stats-aside--collapsed" },

            // Toggle button
            button {
                class: "pwd-stats-aside__toggle",
                onclick: move |_| is_expanded.toggle(),
                aria_label: if is_expanded() { "Collapse statistics panel" } else { "Expand statistics panel" },
                div {
                    class: "pwd-stats-aside__toggle-icon",
                    if is_expanded() {
                        // Chevron left (collapse)
                        svg {
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M15 18l-6-6 6-6" }
                        }
                    } else {
                        // Chevron right (expand)
                        svg {
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M9 18l6-6-6-6" }
                        }
                    }
                }
            }

            // Content wrapper per animazione
            div { class: "pwd-stats-aside__content",

                // Header visibile solo quando espanso
                div { class: "pwd-stats-aside__header",
                    h3 { class: "pwd-stats-aside__title", "Password Stats" }
                    p { class: "pwd-stats-aside__subtitle", "Click to filter" }
                }

                // Grid delle statistiche
                div { class: "pwd-stats-aside__grid",
                    for (initial, count, label, variant, strength) in stat_items {
                        {
                            let strength_clone = strength;
                            rsx! {
                                div {
                                    key: "{label}",
                                    class: format!(
                                        "{} {}",
                                        if is_expanded() {
                                            "pwd-stats-aside__item pwd-stats-aside__item--expanded"
                                        } else {
                                            "pwd-stats-aside__item pwd-stats-aside__item--collapsed"
                                        },
                                        if active_filter == strength {
                                            "pwd-stats-aside__item--active"
                                        } else {
                                            ""
                                        }
                                    ),
                                    onclick: move |_| {
                                        on_stat_click.call(strength_clone);
                                    },

                                    // Badge iniziale (sempre visibile)
                                    div {
                                        class: variant.badge_class(),
                                        span { class: "pwd-stats-aside__initial", "{initial}" }
                                    }

                                    // Contenuto espanso (valore e label)
                                    div { class: "pwd-stats-aside__details",
                                        span { class: "pwd-stats-aside__value", "{count}" }
                                        span { class: "pwd-stats-aside__label", "{label}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
