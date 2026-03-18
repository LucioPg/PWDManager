# Theme Toggle - Dark/Light Mode

## Obiettivo

Aggiungere un toggle nella tab "Aspect" dei Settings per cambiare tema tra Dark e Light. Il cambio deve essere visibile immediatamente (prima del save) e persistere al riavvio tramite DB.

## Architettura

### Componenti coinvolti

| Componente | Responsabilita |
|---|---|
| `App` | Fornisce `Signal<Theme>` via `use_context_provider` (default `Light`) |
| `AuthWrapper` | Dopo il login, fa fetch di `fetch_user_settings(user_id)`, aggiorna il Signal globale una volta sola |
| `RouteWrapper` | Legge il Signal globale e applica `data-theme` sul div root |
| `SettingsTabContent` | Renderizza `AspectSettings {}` (nessun prop) |
| `AspectSettings` | Self-contained: toggle modifica Signal globale, Save persiste nel DB |

### Flusso dati

```
DB --fetch_user_settings--> AuthWrapper aggiorna Signal<Theme>
                                      |
Toggle in AspectSettings --> modifica Signal<Theme> --> RouteWrapper applica data-theme
                                      |
Save button --> UserSettings::upsert_by_id --> DB
```

## Dettagli implementativi

### Signal globale (App)

- `Signal<Theme>` con default `Theme::Light`
- Fornito via `use_context_provider` in `App`, allo stesso livello di `SqlitePool` e `AuthState`

### Fetch iniziale (AuthWrapper)

- `use_resource` con `fetch_user_settings(pool, user_id)`
- Pattern idempotente: fetch solo la prima volta che l'utente e autenticato (usare un flag signal `theme_fetched: bool`)
- Se il fetch fallisce: mantieni `Light` come default, mostra toast di errore
- Se il fetch restituisce `None` (nessun record): mantenere il default `Light`
- Se il fetch restituisce `Some(settings)`: aggiornare il Signal globale con `settings.theme`

### Propagazione visiva (RouteWrapper)

- Leggere il Signal globale dal context
- Impostare `data-theme` attribute sul div root:
  - `Theme::Light` -> `data-theme="light"`
  - `Theme::Dark` -> `data-theme="dark"`
- Nessun CSS in questo task (gestito separatamente)

### AspectSettings (self-contained, zero props)

- Props: nessuna (il componente e self-contained come `StoredPasswordSettings`)
- Legge `Signal<Theme>` dal context per lo stato del toggle
- `use_resource` per fetch di `fetch_user_settings(user_id)` -> serve per ottenere l'`id` del record (per upsert)
- Toggle: `checked = (theme == Light)`, onchange aggiorna il Signal globale
- Pulsante Save:
  - Costruisce `UserSettings` con l'`id` dal fetch (se esiste), `user_id`, `theme` dal Signal
  - Chiama `UserSettings::upsert_by_id(&settings, pool)`
  - Toast di successo/errore

### SettingsTabContent

- Sostituire il placeholder nella tab "Aspect" con `AspectSettings {}`

## Constraint

- Non usare `dx` (solo `cargo check` / `cargo test`)
- Nessun CSS in questo task
- Toggle di pwd-dioxus: `use pwd_dioxus::{Toggle, ToggleColor, ToggleSize}`
- `UserSettings::upsert_by_id` e `fetch_user_settings` gia esistenti in `db_backend.rs`
