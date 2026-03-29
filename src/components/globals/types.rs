// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#[derive(Clone, PartialEq, Copy)]
pub enum TableOrder {
    AZ,
    ZA,
    Oldest,
    Newest,
}

impl TableOrder {
    /// Restituisce la clausola SQL ORDER BY corrispondente.
    pub fn order_by_clause(&self) -> &'static str {
        match self {
            TableOrder::AZ => "name ASC",
            TableOrder::ZA => "name DESC",
            TableOrder::Newest => "created_at DESC",
            TableOrder::Oldest => "created_at ASC",
        }
    }
}
