// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;

#[component]
pub fn BulkActionBar(
    count: usize,
    on_move: EventHandler<()>,
    on_clone: EventHandler<()>,
    on_clear: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "pwd-bulk-action-bar",
            span { class: "pwd-bulk-action-bar-count",
                "{count} selected"
            }
            button {
                class: "btn btn-sm btn-primary",
                r#type: "button",
                onclick: move |_| on_move.call(()),
                "Move to..."
            }
            button {
                class: "btn btn-sm btn-secondary",
                r#type: "button",
                onclick: move |_| on_clone.call(()),
                "Clone to..."
            }
            button {
                class: "btn btn-sm btn-ghost",
                r#type: "button",
                onclick: move |_| on_clear.call(()),
                "Clear selection"
            }
        }
    }
}
