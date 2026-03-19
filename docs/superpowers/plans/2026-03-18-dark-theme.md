# Dark Theme Completamento - Piano di Implementazione

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fornire dark theme completo a tutti i componenti dell'app, risolvendo il bug architetturale per cui il toggle dark/light non funzionava su quasi nessun componente.

**Architecture:** Fix del meccanismo CSS `--dark`/`--light` per rispondere a `data-theme` invece che a `@media (prefers-color-scheme)`. Sostituzione di colori hardcoded con varianti DaisyUI theme-aware (`bg-base-100`, `text-base-content`, ecc.). Override `[data-theme="dark"]` per classi CSS custom che non possono usare variabili DaisyUI.

**Tech Stack:** Tailwind CSS v4, DaisyUI 5, Dioxus 0.7 (RSX)

**Nota:** Non applicabile TDD — la verifica è visiva tramite `dx serve --desktop`.

---

### Task 1: Fix del meccanismo `--dark`/`--light` + DaisyUI classi scaffold

**Files:**
- Modify: `assets/input_main.css:26-38` (fix variabili)
- Modify: `assets/input_main.css` (aggiungere classi DaisyUI scaffold alla fine)

- [ ] **Step 1: Sostituire il meccanismo `@media (prefers-color-scheme)` con `[data-theme="dark"]`**

In `assets/input_main.css`, righe 26-38, sostituire:

```css
@media (prefers-color-scheme: dark) {
    :root {
        --dark: initial;
        --light: ;
    }
}

@media (prefers-color-scheme: light) {
    :root {
        --dark: ;
        --light: initial;
    }
}
```

Con:

```css
:root {
    --dark: ;
    --light: initial;
}

[data-theme="dark"] {
    --dark: initial;
    --light: ;
}
```

Questo fa funzionare TUTTE le variabili `var(--dark, X) var(--light, Y)` definite in `@theme` con il toggle dell'app.

- [ ] **Step 2: Aggiungere scaffold delle classi DaisyUI theme-aware**

Alla fine di `assets/input_main.css` (dopo l'ultima riga), aggiungere:

```css
/* ============================================================
   DAISYUI THEME-AWARE CLASSES - Used by dark mode overrides
   Tailwind generates CSS only for classes found in source files.
   These classes are used in `[data-theme="dark"]` overrides below.
   ============================================================ */
@layer components {
    .pwd-theme-bg-base-100 { @apply bg-base-100; }
    .pwd-theme-bg-base-200 { @apply bg-base-200; }
    .pwd-theme-bg-base-300 { @apply bg-base-300; }
    .pwd-theme-text-base-content { @apply text-base-content; }
    .pwd-theme-text-base-content-10 { @apply text-base-content/10; }
    .pwd-theme-text-base-content-40 { @apply text-base-content/40; }
    .pwd-theme-text-base-content-50 { @apply text-base-content/50; }
    .pwd-theme-text-base-content-60 { @apply text-base-content/60; }
    .pwd-theme-text-base-content-70 { @apply text-base-content/70; }
    .pwd-theme-text-base-content-80 { @apply text-base-content/80; }
    .pwd-theme-border-base-200 { @apply border-base-200; }
    .pwd-theme-border-base-300 { @apply border-base-300; }
    .pwd-theme-hover-bg-base-200 { @apply hover:bg-base-200; }
    .pwd-theme-hover-text-base-content { @apply hover:text-base-content; }
    .pwd-theme-text-error-content { @apply text-error-content; }
    .pwd-theme-text-error { @apply text-error; }
    .pwd-theme-text-success { @apply text-success; }
    .pwd-theme-text-warning { @apply text-warning; }
    .pwd-theme-text-info { @apply text-info; }
    .pwd-theme-hover-bg-error-10 { @apply hover:bg-error/10; }
    .pwd-theme-hover-bg-success-10 { @apply hover:bg-success-10; }
    .pwd-theme-text-primary-400 { @apply text-primary-400; }
    .pwd-theme-border-primary-500 { @apply border-primary-500; }
    .pwd-theme-hover-border-primary-400 { @apply hover:border-primary-400; }
}
```

Queste classi `pwd-theme-*` servono solo a forzare Tailwind a generare il CSS per le classi DaisyUI theme-aware. Le usiamo come `@apply` nelle regole `[data-theme="dark"]` del Task 3.

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "fix: switch --dark/--light CSS vars from prefers-color-scheme to data-theme"
```

---

### Task 2: Spostare `@media (prefers-color-scheme: dark)` in `[data-theme="dark"]`

**Files:**
- Modify: `assets/input_main.css:1671-1735` (stats aside)
- Modify: `assets/input_main.css:1823-1832` (table)

- [ ] **Step 1: Stats aside dark overrides**

In `assets/input_main.css`, righe 1671-1735, sostituire:

```css
@media (prefers-color-scheme: dark) {
```

Con:

```css
[data-theme="dark"] {
```

E rimuovere la `}` di chiusura corrispondente (riga 1735).

- [ ] **Step 2: Table dark overrides**

In `assets/input_main.css`, righe 1823-1832, sostituire:

```css
@media (prefers-color-scheme: dark) {
    .pwd-table tbody tr:hover {
        background-color: rgba(255, 255, 255, 0.03);
    }

    .pwd-table th {
        color: var(--secondary-color-3, #dcdcdc);
        border-bottom-color: var(--primary-color-5, #262626);
    }
}
```

Con:

```css
[data-theme="dark"] .pwd-table tbody tr:hover {
    background-color: rgba(255, 255, 255, 0.03);
}

[data-theme="dark"] .pwd-table th {
    color: var(--secondary-color-3, #dcdcdc);
    border-bottom-color: var(--primary-color-5, #262626);
}
```

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "fix: move dark mode overrides from prefers-color-scheme to data-theme"
```

---

### Task 3: Aggiungere override `[data-theme="dark"]` per classi CSS hardcoded

**Files:**
- Modify: `assets/input_main.css` (aggiungere blocco alla fine del file)

- [ ] **Step 1: Aggiungere il blocco completo di override dark**

Alla fine di `assets/input_main.css` (dopo lo scaffold del Task 1), aggiungere:

```css
/* ============================================================
   DARK THEME OVERRIDES - [data-theme="dark"]
   Tutti gli stili che hanno colori hardcoded in light mode
   ricevono override per il tema dark tramite variabili DaisyUI.
   ============================================================ */

[data-theme="dark"] {

    /* ---- Auth Form ---- */
    .auth-form,
    .auth-form-lg,
    .auth-form-centered,
    .auth-form-tabbed {
        @apply pwd-theme-bg-base-100 pwd-theme-border-base-300;
    }

    /* ---- Cards ---- */
    .card {
        @apply pwd-theme-bg-base-100 pwd-theme-border-base-300;
    }

    .stat-card {
        @apply pwd-theme-bg-base-100 pwd-theme-border-base-300;
    }
    .stat-card:hover {
        border-color: var(--color-base-200);
    }

    /* ---- Inputs ---- */
    .pwd-input {
        @apply pwd-theme-bg-base-100 pwd-theme-border-base-300 pwd-theme-text-base-content;
        --tw-placeholder-opacity: 0.4;
        color: var(--color-base-content);
    }
    .pwd-input::placeholder {
        color: var(--color-base-content);
    }
    .pwd-input:hover {
        border-color: var(--color-base-200);
    }
    .pwd-input:disabled {
        @apply pwd-theme-bg-base-200 pwd-theme-text-base-content-40;
    }

    /* ---- Navbar ---- */
    .navbar {
        border-color: var(--color-base-300);
    }

    .navbar-brand-text {
        @apply pwd-theme-text-base-content;
    }

    .navbar-link {
        @apply pwd-theme-text-base-content-70;
    }
    .navbar-link:hover {
        @apply pwd-theme-hover-bg-base-200 pwd-theme-hover-text-base-content;
    }

    .navbar-user {
        border-color: var(--color-base-300);
    }

    /* ---- Typography ---- */
    .text-h2,
    .text-h3 {
        @apply pwd-theme-text-base-content;
    }

    .text-body {
        @apply pwd-theme-text-base-content-70;
    }

    .text-error {
        @apply pwd-theme-text-error-content;
    }

    .form-label {
        @apply pwd-theme-text-base-content;
    }

    .form-label-required::after {
        @apply pwd-theme-text-error-content;
    }

    /* ---- Avatar ---- */
    .avatar-bordered {
        border-color: var(--color-base-200);
    }
    .avatar-hover {
        border-color: var(--color-base-200);
    }
    .avatar-hover:hover {
        border-color: var(--color-primary);
    }

    /* ---- Slogan ---- */
    .pwd-slogan-text {
        @apply pwd-theme-text-base-content;
    }

    /* ---- Error page ---- */
    .error-code {
        @apply pwd-theme-text-base-content-10;
    }

    /* ---- Password visibility toggle ---- */
    .password-visibility-toggle {
        @apply pwd-theme-text-base-content-50;
    }
    .password-visibility-toggle:hover {
        @apply pwd-theme-text-base-content-80 pwd-theme-hover-bg-base-200;
    }
    .password-visibility-toggle:disabled:hover {
        background: transparent;
    }

    /* ---- Buttons ---- */
    .btn-secondary {
        @apply pwd-theme-bg-base-100 pwd-theme-text-primary-400 pwd-theme-border-primary-500;
    }
    .btn-secondary:hover {
        @apply pwd-theme-hover-bg-base-200;
    }

    .btn-ghost {
        @apply pwd-theme-text-primary-400;
    }
    .btn-ghost:hover {
        @apply pwd-theme-hover-bg-base-200;
    }

    /* ---- Row action buttons ---- */
    .pwd-burger-btn {
        @apply pwd-theme-text-base-content-50;
    }
    .pwd-burger-btn:hover {
        @apply pwd-theme-text-base-content-80 pwd-theme-hover-bg-base-200;
    }
    .pwd-burger-btn:focus {
        --tw-ring-color: var(--color-base-content);
    }

    .pwd-delete-btn {
        @apply pwd-theme-text-base-content-50;
    }
    .pwd-delete-btn:hover {
        @apply pwd-theme-text-error pwd-theme-hover-bg-error-10;
    }

    /* ---- Tooltips ---- */
    .pwd-row-tooltip h4 {
        color: var(--color-base-content);
    }
    .pwd-row-tooltip p {
        color: var(--color-base-content);
        opacity: 0.8;
    }

    .strength-reasons-tooltip h4 {
        color: var(--color-base-content);
    }
    .strength-reasons-tooltip li {
        color: var(--color-base-content);
        opacity: 0.8;
    }

    /* ---- Strength cursor ---- */
    .strength-cursor {
        background-color: var(--color-base-200);
        border-color: var(--color-base-content);
        border-color: color-mix(in srgb, var(--color-base-content) 40%, transparent);
    }

    /* ---- Tabs trigger (non-active) ---- */
    .tabs-trigger {
        background: var(--color-base-200);
        color: var(--color-base-content);
        border-color: var(--color-base-300);
    }

    /* ---- Tab shadow ---- */
    [data-variant="default"] .tabs-list {
        background: var(--color-base-300);
    }

    /* ---- Search input ---- */
    .pwd-search-input {
        @apply pwd-theme-bg-base-100 pwd-theme-border-base-300 pwd-theme-text-base-content;
    }
    .pwd-search-input::placeholder {
        color: var(--color-base-content);
        opacity: 0.4;
    }
    .pwd-search-input:focus {
        outline-color: var(--color-primary);
        box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 15%, transparent);
    }
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `dx serve --desktop`

Verificare che il CSS compila senza errori. Se ci sono errori su classi DaisyUI non trovate, aggiungerle allo scaffold del Task 1 Step 2.

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "feat: add comprehensive dark theme overrides for all CSS classes"
```

---

### Task 4: Aggiornare classi RSX hardcoded - Dialoghi error

**Files:**
- Modify: `src/components/globals/secret_notes_tooltip/component.rs:24,29,33,53,54,60`
- Modify: `src/components/features/logout.rs:29`
- Modify: `src/components/features/upsert_user.rs:395`
- Modify: `src/components/globals/dialogs/user_deletion.rs:63,88`
- Modify: `src/components/globals/dialogs/stored_password_deletion.rs:59,72`
- Modify: `src/components/globals/dialogs/stored_all_passwords_deletion.rs:54,66`
- Modify: `src/components/globals/dialogs/migration_warning.rs:58,73`

- [ ] **Step 1: secret_notes_tooltip - text-gray-* → text-base-content/**

In `src/components/globals/secret_notes_tooltip/component.rs`:

| Riga | Vecchio | Nuovo |
|------|---------|-------|
| 24 | `text-gray-600` | `text-base-content/60` |
| 29 | `text-gray-700` | `text-base-content/80` |
| 33 | `text-gray-500` | `text-base-content/50` |
| 53 | `text-gray-600` | `text-base-content/60` |
| 54 | `text-gray-700` | `text-base-content/80` |
| 60 | `text-gray-500` | `text-base-content/50` |

- [ ] **Step 2: logout.rs - text-error-600 → text-error**

In `src/components/features/logout.rs:29`:

Vecchio: `"w-16 h-16 text-error-600 mx-auto mb-4"`
Nuovo: `"w-16 h-16 text-error mx-auto mb-4"`

- [ ] **Step 3: upsert_user.rs - error classes**

In `src/components/features/upsert_user.rs:395`:

Vecchio: `"text-error-600 hover:bg-error-50 hover:text-error-700"`
Nuovo: `"text-error hover:bg-error/10"`

- [ ] **Step 4: user_deletion.rs**

In `src/components/globals/dialogs/user_deletion.rs`:

Riga 63: `class: "text-error-600 py-2"` → `class: "text-error py-2"`
Riga 88: `additional_class: "text-error-600 hover:bg-error-50".to_string()` → `additional_class: "text-error hover:bg-error/10".to_string()`

- [ ] **Step 5: stored_password_deletion.rs**

In `src/components/globals/dialogs/stored_password_deletion.rs`:

Riga 59: `class: "text-error-600 py-2"` → `class: "text-error py-2"`
Riga 72: `additional_class: "text-error-600 hover:bg-error-50".to_string()` → `additional_class: "text-error hover:bg-error/10".to_string()`

- [ ] **Step 6: stored_all_passwords_deletion.rs**

In `src/components/globals/dialogs/stored_all_passwords_deletion.rs`:

Riga 54: `class: "text-error-600 py-2"` → `class: "text-error py-2"`
Riga 66: `additional_class: "text-error-600 hover:bg-error-50".to_string()` → `additional_class: "text-error hover:bg-error/10".to_string()`

- [ ] **Step 7: migration_warning.rs**

In `src/components/globals/dialogs/migration_warning.rs`:

Riga 58: `class: "text-error-600 py-2"` → `class: "text-error py-2"`
Riga 73: `additional_class: "text-error-600 hover:bg-error-50".to_string()` → `additional_class: "text-error hover:bg-error/10".to_string()`

- [ ] **Step 8: Verificare compilazione**

Run: `dx serve --desktop`

- [ ] **Step 9: Commit**

```bash
git add src/components/
git commit -m "feat: replace hardcoded gray/error colors with DaisyUI theme-aware classes"
```

---

### Task 5: Aggiornare classi RSX hardcoded - Success, Warning, Info, Spinner

**Files:**
- Modify: `src/components/globals/dialogs/stored_password_upsert.rs:245`
- Modify: `src/components/globals/dialogs/stored_password_show.rs:168`
- Modify: `src/components/globals/dialogs/export_warning.rs:72`
- Modify: `src/components/globals/dialogs/import_warning.rs:66,72`
- Modify: `src/components/globals/dialogs/export_progress.rs:41`
- Modify: `src/components/globals/dialogs/import_progress.rs:41`
- Modify: `src/components/globals/dialogs/migration_progress.rs:36`
- Modify: `src/components/features/dashboard.rs:321`
- Modify: `src/components/globals/table/table.rs:119`
- Modify: `src/components/features/storedpassword_settings.rs:139`

- [ ] **Step 1: Success classes**

`src/components/globals/dialogs/stored_password_upsert.rs:245`:
Vecchio: `"text-success-600 hover:bg-success-50".to_string()`
Nuovo: `"text-success hover:bg-success/10".to_string()`

`src/components/globals/dialogs/stored_password_show.rs:168`:
Vecchio: `"text-success-600 hover:bg-success-50".to_string()`
Nuovo: `"text-success hover:bg-success/10".to_string()`

- [ ] **Step 2: Warning classes**

`src/components/globals/dialogs/export_warning.rs:72`:
Vecchio: `class: "text-warning-600 py-2"`
Nuovo: `class: "text-warning py-2"`

`src/components/globals/dialogs/export_progress.rs:41`:
Vecchio: `class: "text-warning-600 py-2"`
Nuovo: `class: "text-warning py-2"`

`src/components/globals/dialogs/import_progress.rs:41`:
Vecchio: `class: "text-warning-600 py-2"`
Nuovo: `class: "text-warning py-2"`

`src/components/globals/dialogs/migration_progress.rs:36`:
Vecchio: `class: "text-warning-600 py-2"`
Nuovo: `class: "text-warning py-2"`

- [ ] **Step 3: Warning + Info classes**

`src/components/globals/dialogs/import_warning.rs:66`:
Vecchio: `class: "text-warning-600 py-2"`
Nuovo: `class: "text-warning py-2"`

`src/components/globals/dialogs/import_warning.rs:72`:
Vecchio: `class: "text-info-600 py-2"`
Nuovo: `class: "text-info py-2"`

- [ ] **Step 4: Spinner color classes**

`src/components/features/dashboard.rs:321`:
Vecchio: `Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }`
Nuovo: `Spinner { size: SpinnerSize::Medium, color_class: "text-info" }`

`src/components/globals/table/table.rs:119`:
Vecchio: `Spinner { size: SpinnerSize::Large, color_class: "text-blue-500" }`
Nuovo: `Spinner { size: SpinnerSize::Large, color_class: "text-info" }`

`src/components/features/storedpassword_settings.rs:139`:
Vecchio: `Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }`
Nuovo: `Spinner { size: SpinnerSize::Medium, color_class: "text-info" }`

- [ ] **Step 5: Verificare compilazione**

Run: `dx serve --desktop`

- [ ] **Step 6: Commit**

```bash
git add src/components/
git commit -m "feat: replace hardcoded success/warning/info/blue colors with DaisyUI theme-aware classes"
```

---

### Task 6: Verifica visiva completa

- [ ] **Step 1: Avviare l'app in dev**

Run: `dx serve --desktop`

- [ ] **Step 2: Test light mode (baseline)**

1. Login con credenziali
2. Verificare che l'aspetto light-mode sia IDENTICO a prima delle modifiche
3. Navbar, form login, dashboard, stats aside, tabella, tooltip notes, slogan
4. Se qualcosa è cambiato, notare cosa e correggere

- [ ] **Step 3: Test dark mode toggle**

1. Settings → Aspect → togglare Dark
2. Verificare: navbar (bordo, testo), auth forms, dashboard, stat cards, stats aside (sfondo scuro), tabella (header, hover), tooltip notes, tooltip strength, password visibility toggle, slogan, tutti i dialoghi (delete, upsert, export, import, migration), toast, bottoni secondary/ghost, error page, pagination
3. Tornare a Light e verificare che tutto torni normale
4. Ripetere il toggle più volte per verificare transizione fluida

- [ ] **Step 4: Fix eventuali problemi trovati**

Correggere qualsiasi componente che non si adatta correttamente al dark theme.

- [ ] **Step 5: Commit finale**

```bash
git add -A
git commit -m "fix: dark theme visual fixes after verification"
```
