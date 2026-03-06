use super::PaginationState;
use dioxus::prelude::*;

/// Calcola le pagine visibili nei controlli.
///
/// Mostra max 5 numeri di pagina, centrati sulla pagina corrente.
/// Esempio: se current=5, total=10 → [3, 4, 5, 6, 7]
fn calculate_visible_pages(current: usize, total: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }

    if total <= 5 {
        return (0..total).collect();
    }

    // Centra la pagina corrente, ma rispetta i limiti
    let start = current.saturating_sub(2);
    let end = (start + 5).min(total);
    let start = end.saturating_sub(5);

    (start..end).collect()
}

/// Controlli di paginazione con DaisyUI.
///
/// Visualizza: [«] [1] [2] [3] [4] [5] [»]
/// + info "Page X of Y (Z items)"
#[component]
pub fn PaginationControls(
    /// Stato di paginazione (da context o props)
    pagination: PaginationState,
    /// Callback chiamato quando si cambia pagina
    on_page_change: Callback<usize>,
) -> Element {
    let current_page = pagination.current_page();
    let total_pages = pagination.total_pages();
    let total_count = pagination.total_count();
    let is_loading = pagination.is_loading();

    let visible_pages = calculate_visible_pages(current_page, total_pages);
    let has_prev = pagination.has_prev();
    let has_next = pagination.has_next();

    rsx! {
        div { class: "pwd-pagination-container mt-4",
            // Controlli paginazione
            div { class: "join justify-center w-full flex",
                // Previous button
                button {
                    class: "join-item btn btn-sm",
                    disabled: !has_prev || is_loading,
                    onclick: move |_| {
                        if has_prev {
                            on_page_change.call(current_page.saturating_sub(1));
                        }
                    },
                    "«"
                }

                // Page numbers
                for page_num in visible_pages {
                    {
                        let is_current = page_num == current_page;
                        let page_for_closure = page_num;
                        rsx! {
                            button {
                                class: if is_current {
                                    "join-item btn btn-sm btn-primary"
                                } else {
                                    "join-item btn btn-sm"
                                },
                                disabled: is_loading,
                                onclick: move |_| {
                                    on_page_change.call(page_for_closure);
                                },
                                "{page_num + 1}"  // Display 1-indexed
                            }
                        }
                    }
                }

                // Next button
                button {
                    class: "join-item btn btn-sm",
                    disabled: !has_next || is_loading,
                    onclick: move |_| {
                        if has_next {
                            on_page_change.call(current_page + 1);
                        }
                    },
                    "»"
                }
            }

            // Page info
            div { class: "text-center text-sm mt-2 opacity-70",
                if total_pages > 0 {
                    "Page {current_page + 1} of {total_pages} ({total_count} items)"
                } else {
                    "No items"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_visible_pages_empty() {
        assert_eq!(calculate_visible_pages(0, 0), Vec::<usize>::new());
    }

    #[test]
    fn test_calculate_visible_pages_single_page() {
        // Singola pagina → [0]
        assert_eq!(calculate_visible_pages(0, 1), vec![0]);
    }

    #[test]
    fn test_calculate_visible_pages_exactly_5() {
        // Esattamente 5 pagine → mostra tutte
        assert_eq!(calculate_visible_pages(0, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(calculate_visible_pages(2, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(calculate_visible_pages(4, 5), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_calculate_visible_pages_less_than_5() {
        assert_eq!(calculate_visible_pages(0, 3), vec![0, 1, 2]);
        assert_eq!(calculate_visible_pages(2, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_calculate_visible_pages_middle() {
        // Pagina 5 di 10 → [3, 4, 5, 6, 7]
        assert_eq!(calculate_visible_pages(5, 10), vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_calculate_visible_pages_near_start() {
        // Pagina 1 di 10 → [0, 1, 2, 3, 4]
        assert_eq!(calculate_visible_pages(1, 10), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_calculate_visible_pages_near_end() {
        // Pagina 9 di 10 → [5, 6, 7, 8, 9]
        assert_eq!(calculate_visible_pages(9, 10), vec![5, 6, 7, 8, 9]);
    }
}
