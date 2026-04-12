// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;
use pwd_types::{PasswordStrength, StoredRawPassword};
use std::collections::HashMap;

/// Chiave della cache: (filtro attivo, numero pagina)
pub type CacheKey = (Option<PasswordStrength>, usize);

/// Stato globale della paginazione.
///
/// Gestisce pagina corrente, filtro, cache e loading state.
/// Tutti i campi sono Signal per la reattività Dioxus.
#[derive(Clone, Copy, PartialEq)]
pub struct PaginationState {
    /// Pagina corrente (0-indexed, display come 1-indexed)
    pub current_page: Signal<usize>,

    /// Dimensione pagina (default: 20)
    pub page_size: Signal<usize>,

    /// Filtro attivo per PasswordStrength
    pub active_filter: Signal<Option<PasswordStrength>>,

    /// Cache: (filter, page) → passwords
    /// Mantiene pagine già caricate per evitare re-fetch
    pub cache: Signal<HashMap<CacheKey, Vec<StoredRawPassword>>>,

    /// Totale risultati per il filtro corrente (dal DB)
    pub total_count: Signal<u64>,

    /// Loading state per mostrare spinner
    pub is_loading: Signal<bool>,
}

#[allow(dead_code)]
impl PaginationState {
    /// Crea un nuovo stato di paginazione con page_size di default (20).
    ///
    /// **IMPORTANTE:** Questo metodo deve essere chiamato all'interno di
    /// `use_context_provider()` per garantire che i Signal siano gestiti
    /// correttamente dal lifecycle di Dioxus.
    ///
    /// # Example
    /// ```rust
    /// let mut pagination = use_context_provider(|| PaginationState::new());
    /// ```
    pub fn new() -> Self {
        Self {
            current_page: Signal::new(0),
            page_size: Signal::new(20),
            active_filter: Signal::new(None),
            cache: Signal::new(HashMap::new()),
            total_count: Signal::new(0),
            is_loading: Signal::new(false),
        }
    }

    /// Crea un nuovo stato con page_size personalizzato.
    pub fn with_page_size(page_size: usize) -> Self {
        Self {
            page_size: Signal::new(page_size),
            ..Self::new()
        }
    }

    /// Invalida tutta la cache.
    ///
    /// Chiamare dopo operazioni CRUD (create/update/delete)
    /// per forzare il refresh dei dati.
    pub fn invalidate(&mut self) {
        self.cache.write().clear();
    }

    /// Imposta una nuova dimensione pagina e invalida la cache.
    ///
    /// La cache viene invalidata perché le pagine cached
    /// avrebbero dimensione diversa dal nuovo page_size.
    pub fn set_page_size(&mut self, page_size: usize) {
        if *self.page_size.read() != page_size {
            self.page_size.set(page_size);
            self.cache.write().clear(); // Invalida cache per evitare dimensioni mismatch
            self.current_page.set(0); // Reset a prima pagina
        }
    }

    /// Imposta un nuovo filtro e resetta a pagina 0.
    ///
    /// NON invalida la cache - le pagine già caricate
    /// per altri filtri rimangono disponibili.
    pub fn set_filter(&mut self, filter: Option<PasswordStrength>) {
        if *self.active_filter.read() != filter {
            self.active_filter.set(filter);
            self.current_page.set(0);
        }
    }

    /// Vai a una pagina specifica.
    ///
    /// Non fa nulla se la pagina è fuori range o uguale alla corrente.
    ///
    /// **WARNING:** Non chiamare questo metodo dentro un `use_effect` che non
    /// dovrebbe dipendere da `current_page`. Internamente legge `current_page`,
    /// creando una dipendenza reattiva. Per resettare la pagina in un effetto,
    /// usare `pagination.current_page.set(0)` direttamente.
    pub fn go_to_page(&mut self, page: usize) {
        if page != *self.current_page.read() {
            self.current_page.set(page);
        }
    }

    /// Pagina successiva (se disponibile)
    pub fn next_page(&mut self) {
        let current = *self.current_page.read();
        let total_pages = self.total_pages();
        if current + 1 < total_pages {
            self.current_page.set(current + 1);
        }
    }

    /// Pagina precedente (se disponibile)
    pub fn prev_page(&mut self) {
        let current = *self.current_page.read();
        if current > 0 {
            self.current_page.set(current - 1);
        }
    }

    /// Calcola numero totale di pagine
    pub fn total_pages(&self) -> usize {
        let total = self.total_count() as usize;
        let page_size = self.page_size();
        if page_size == 0 {
            return 0;
        }
        total.div_ceil(page_size)
    }

    /// Verifica se può andare a pagina precedente
    pub fn has_prev(&self) -> bool {
        self.current_page() > 0
    }

    /// Verifica se può andare a pagina successiva
    pub fn has_next(&self) -> bool {
        let current = self.current_page();
        current + 1 < self.total_pages()
    }

    /// Ottiene la chiave cache corrente
    pub fn current_cache_key(&self) -> CacheKey {
        (self.active_filter(), self.current_page())
    }

    /// Verifica se la pagina corrente è in cache
    pub fn is_current_page_cached(&self) -> bool {
        let key = self.current_cache_key();
        self.cache.read().contains_key(&key)
    }

    /// Ottiene la pagina corrente dalla cache (se presente)
    pub fn get_current_page_from_cache(&self) -> Option<Vec<StoredRawPassword>> {
        let key = self.current_cache_key();
        self.cache.read().get(&key).cloned()
    }

    /// Salva una pagina in cache
    pub fn cache_page(
        &mut self,
        filter: Option<PasswordStrength>,
        page: usize,
        passwords: Vec<StoredRawPassword>,
    ) {
        let key = (filter, page);
        self.cache.write().insert(key, passwords);
    }

    // === Getter methods ===

    /// Ottiene la pagina corrente (0-indexed)
    pub fn current_page(&self) -> usize {
        *self.current_page.read()
    }

    /// Ottiene la dimensione pagina
    pub fn page_size(&self) -> usize {
        *self.page_size.read()
    }

    /// Ottiene il filtro attivo
    pub fn active_filter(&self) -> Option<PasswordStrength> {
        *self.active_filter.read()
    }

    /// Ottiene il conteggio totale
    pub fn total_count(&self) -> u64 {
        *self.total_count.read()
    }

    /// Verifica se è in caricamento
    pub fn is_loading(&self) -> bool {
        *self.is_loading.read()
    }
}
