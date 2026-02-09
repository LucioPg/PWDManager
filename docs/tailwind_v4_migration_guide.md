# Guida alla Migrazione: Tailwind CSS v3 → v4

> **Nota importante:** Questa migrazione è particolarmente rilevante per progetti Dioxus che desiderano utilizzare librerie come `dioxus-components`, che richiedono Tailwind v4.

## Panoramica

La migrazione da Tailwind v3 a v4 non è considerata difficile per la maggior parte dei progetti, grazie a uno strumento di aggiornamento automatico, ma introduce cambiamenti strutturali importanti che richiedono attenzione.

---

## Punti Chiave della Migrazione

### Tool Automatico

Puoi usare il comando seguente per automatizzare gran parte del lavoro, inclusi l'aggiornamento delle dipendenze e la conversione del file di configurazione:

```bash
npx @tailwindcss/upgrade
```

Questo tool:
- Aggiorna le dipendenze nel `package.json`
- Converte il file `tailwind.config.js` (se presente)
- Aggiorna le direttive CSS nel tuo file principale

### Configurazione CSS-First

La differenza principale è il passaggio dal file `tailwind.config.js` a una configurazione basata direttamente su **variabili CSS** all'interno del tuo file principale tramite la direttiva `@theme`.

**Esempio v3 (tailwind.config.js):**
```javascript
module.exports = {
  theme: {
    extend: {
      colors: {
        primary: '#3b82f6',
      }
    }
  }
}
```

**Esempio v4 (CSS):**
```css
@import "tailwindcss";

@theme {
  --color-primary: #3b82f6;
}
```

### Rilevamento Contenuti Automatico

Non è più necessario configurare manualmente l'array `content`. Tailwind v4 rileva automaticamente i file (inclusi i `.rs` in Dioxus).

**v3:**
```javascript
module.exports = {
  content: ['./src/**/*.rs', './src/**/*.html'],
}
```

**v4:** Non richiesto - rilevamento automatico

### Prestazioni

Il nuovo motore (**Oxide**) è significativamente più veloce:
- Build fino a **10 volte più rapide**
- Compilazione CSS più efficiente
- Footprint ridotto del 35%

---

## Possibili Difficoltà

### Browser Support

Tailwind v4 punta a browser moderni:

| Browser | Versione Minima |
|---------|-----------------|
| Safari | 16.4+ |
| Chrome | 111+ |
| Firefox | 128+ |

Se devi supportare versioni molto vecchie, potrebbe essere meglio restare alla v3.4.

### Ridenominazione e Rimozione Utility

Alcune utility sono state rimosse o modificate:

**Utility rimosse:**
- `text-opacity-*` → Usa invece `text-{color}/{opacity}`
- `flex-grow-*` → Usa `grow-*`
- `decoration-slice` → Usa `box-decoration-slice`

**Cambiamenti comportamentali:**
- `border` ora default a `currentColor` invece di `gray-200`
- `ring` ora default a 1px invece di 3px, e usa `currentColor`

Il tool di aggiornamento automatico dovrebbe gestire la maggior parte di queste sostituzioni. Controlla la documentazione ufficiale per l'elenco completo.

### Progetti Molto Personalizzati

Se il tuo `tailwind.config.js` contiene:
- Plugin JavaScript complessi
- Logica personalizzata estrema
- Funzioni dinamiche nel theme

Il tool automatico potrebbe richiedere piccoli interventi manuali.

### ⚠️ Il problema critico: @apply con classi personalizzate

Questo è il **cambiamento più problematico** di Tailwind v4 per progetti con un design system CSS esteso.

**In v3** potevi fare:
```css
.btn-base {
  @apply px-4 py-2 rounded-lg font-semibold;
}

.btn-primary {
  @apply btn-base;  /* ✅ Funziona */
  @apply bg-blue-600 text-white;
}
```

**In v4** questo NON funziona più:
```css
.btn-primary {
  @apply btn-base;  /* ❌ ERRORE: Cannot apply unknown utility class */
  @apply bg-blue-600 text-white;
}
```

**Soluzione:** Devi espandere le utility:
```css
.btn-primary {
  @apply px-4 py-2 rounded-lg font-semibold;  /* Espanso manualmente */
  @apply bg-blue-600 text-white;
}
```

Se hai centinaia di classi CSS che dipendono l'una dall'altra (come in `input_main.css` di PWDManager), dovrai riscrivere il file espandendo tutti i `@apply` con classi personalizzate. Questo può essere un lavoro significativo per progetti grandi.

---

## Come Procedere per Dioxus

### Passo 1: Esegui l'upgrade

Nella root del progetto:

```bash
npx @tailwindcss/upgrade
```

### Passo 2: Aggiorna il file CSS principale

Sostituisci le vecchie direttive nel tuo file CSS principale (`assets/input_main.css`):

**Prima (v3):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

**Dopo (v4):**
```css
@import "tailwindcss";
```

### Passo 3: Configurazione per Dioxus (Source Files)

Se usi componenti esterni o hai bisogno di specificare esplicitamente i sorgenti:

```css
@import "tailwindcss";

@source "../src/**/*.rs";
```

### Passo 4: Verifica il processo di build

Assicurati che il comando di build di Tailwind funzioni ancora. In v4, il processo di build è cambiato:

```bash
# In v4 si usa PostCSS con il plugin Tailwind
npm run build:css
```

---

## Migrazione Manuale per Progetti Complessi

Se il tool automatico `@tailwindcss/upgrade` fallisce o se hai un progetto con molte classi CSS personalizzate che usano `@apply`, segui questi passaggi:

### Passo 1: Aggiorna le dipendenze manualmente

Modifica `package.json`:

```json
{
  "devDependencies": {
    "@tailwindcss/postcss": "^4.1.18",
    "postcss": "^8.5.6",
    "postcss-cli": "^11.0.1"
  }
}
```

Poi esegui:
```bash
npm install
```

### Passo 2: Aggiorna postcss.config.js

Il file `postcss.config.js` deve usare il nuovo plugin:

```javascript
// v3 (vecchio)
module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}

// v4 (nuovo)
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

### Passo 3: Aggiorna gli script di build

In `package.json`, aggiorna gli script per usare PostCSS:

```json
{
  "scripts": {
    "build:css": "npx postcss ./assets/input_main.css -o ./assets/main.css && npx postcss ./assets/input.css -o ./assets/tailwind.css",
    "watch:css": "npx postcss ./assets/input_main.css -o ./assets/main.css --watch"
  }
}
```

### Passo 4: Gestione del problema @apply con classi personalizzate

⚠️ **IMPORTANTE:** In Tailwind v4, `@apply` NON può più essere usato con classi CSS personalizzate definite nello stesso file.

**Esempio del problema:**

```css
/* ❌ NON FUNZIONA IN v4 */
.btn {
  @apply px-6 py-3 font-semibold rounded-lg;
}

.btn-primary {
  @apply btn; /* ERRORE: non può usare .btn */
  @apply bg-primary-600 text-white;
}
```

**Soluzione:** Espandi le utility in ogni classe:

```css
/* ✅ CORRETTO IN v4 */
.btn {
  @apply px-6 py-3 font-semibold rounded-lg;
}

.btn-primary {
  @apply px-6 py-3 font-semibold rounded-lg; /* Espanso da .btn */
  @apply bg-primary-600 text-white;
}
```

Per progetti con molte classi CSS che dipendono l'una dall'altra (come nel file `input_main.css` di PWDManager), potrebbe essere necessario riscrivere completamente le classi espandendo tutti i `@apply` che fanno riferimento ad altre classi personalizzate.

### Passo 5: Converti tailwind.config.js in @theme

Copia il contenuto di `tailwind.config.js` nel blocco `@theme`:

```css
@theme {
  /* Colors */
  --color-primary-50: #eff6ff;
  --color-primary-500: #3b82f6;
  --color-primary-600: #2563eb;

  /* Animations */
  --animate-fade-in: fade-in 0.2s ease-out;
  --animate-slide-in-right: slide-in-right 0.3s ease-out;
}

/* Keyframes devono essere definiti separatamente */
@keyframes fade-in {
  0% { opacity: 0; }
  100% { opacity: 1; }
}
```

### Passo 6: Crea backup dei file originali

Prima di apportare modifiche significative:

```bash
cp tailwind.config.js tailwind.config.js.v3.bak
cp assets/input_main.css assets/input_main.css.v3.bak
```

---

## Checklist di Coerenza (Check Manuale Rapido)

Per essere "sicuro al 100%", controlla questi punti nel tuo codice:

### ✓ 1. Rimosso il vecchio file di configurazione

Hai eliminato (o spostato in backup) `tailwind.config.js`?
- In v4 non è più necessario, a meno che tu non voglia mantenere una compatibilità ibrida.
- 💡 Crea un backup: `mv tailwind.config.js tailwind.config.js.v3.bak`

### ✓ 2. Direttive CSS aggiornate

Il tuo file CSS inizia con `@import "tailwindcss";` invece delle vecchie tre righe?

```css
/* CORRETTO (v4) */
@import "tailwindcss";

@source "../src/**/*.rs";

/* ERRATO (v3) */
@tailwind base;
@tailwind components;
@tailwind utilities;
```

### ✓ 3. Source configuration per Dioxus

Se usi componenti esterni o hai bisogno di specificare i file da scansionare, hai aggiunto la direttiva `@source` nel CSS:

```css
@import "tailwindcss";

@source "../src/**/*.rs";
```

### ✓ 4. Nessun @apply con classi personalizzate

Verifica che nel tuo CSS non ci siano `@apply` che fanno riferimento a classi personalizzate definite nello stesso file:

```css
/* ❌ ERRATO IN v4 */
.btn-primary {
  @apply btn;  /* Non puoi usare .btn qui */
  @apply bg-primary-600;
}

/* ✅ CORRETTO IN v4 */
.btn-primary {
  @apply px-6 py-3 font-semibold rounded-lg;  /* Espanso direttamente */
  @apply bg-primary-600;
}
```

### ✓ 5. postcss.config.js aggiornato

Il tuo `postcss.config.js` usa il plugin corretto?

```javascript
// CORRETTO (v4)
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}

// ERRATO (v3)
module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}
```

### ✓ 6. Build CSS funziona

```bash
npm run build:css
```

Non dovresti vedere errori relativi a classi sconosciute o problemi con `@apply`.

---

## Link Utili

**Documentazione Ufficiale:**
- [Upgrade Guide - Tailwind CSS](https://tailwindcss.com/docs/upgrade-guide)
- [Compatibility Guide - Browser Support](https://tailwindcss.com/docs/compatibility)
- [Functions and Directives - @source e @theme](https://tailwindcss.com/docs/functions-and-directives)
- [Detecting Classes in Source Files](https://tailwindcss.com/docs/detecting-classes-in-source-files)

**Risorse Community:**
- [Tailwind v4.0 Announcement](https://tailwindcss.com/blog/tailwindcss-v4)
- [Browser Support Discussion - GitHub Issue #14119](https://github.com/tailwindlabs/tailwindcss/issues/14119)
- [Real-world Migration Steps - Dev.to](https://dev.to/mridudixit15/real-world-migration-steps-from-tailwind-css-v3-to-v4-1nn3)
- [Migration Guide - Dev.to](https://dev.to/ippatev/migration-guide-tailwind-css-v3-to-v4-f5h)
- [Guide to @source Directive - TailKits](https://tailkits.com/blog/tailwind-at-source-directive/)

---

## Note Aggiuntive per il Progetto PWDManager

Considerando la struttura del progetto:

```
PWDManager/
├── assets/
│   ├── input_main.css    # File principale da aggiornare
│   ├── tailwind.css      # Output generato (utilities base)
│   ├── main.css          # Output generato (classi personalizzate)
│   └── input.css         # Sorgente semplice con solo @import
```

### Stato della Migrazione (Completata)

✅ **Migrazione a Tailwind CSS v4.1.18 completata il 2026-02-09**

**Cambiamenti applicati:**

1. **Dipendenze aggiornate:**
   - `@tailwindcss/postcss`: v4.1.18
   - `postcss-cli`: v11.0.1
   - Rimosso: `autoprefixer` (incluso in v4)

2. **File di configurazione:**
   - `postcss.config.js`: Aggiornato per usare `@tailwindcss/postcss`
   - `tailwind.config.js`: Spostato in `tailwind.config.js.v3.bak` (non più necessario)

3. **File CSS:**
   - `input.css`: Aggiornato con `@import "tailwindcss"`
   - `input_main.css`: Completamente riscritto per v4
     - Aggiunto `@theme` con tutti i colori personalizzati
     - Aggiunto `@source "../src/**/*.rs"` per Dioxus
     - Tutte le classi con `@apply` espanse (nessun riferimento a classi custom)

4. **Backup creati:**
   - `tailwind.config.js.v3.bak`
   - `assets/input_main.css.v3.bak`

### Classi CSS Personalizzate in PWDManager

Il progetto usa un sistema di classi CSS molto esteso. Le principali categorie:

- **Buttons:** `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-danger`, `.btn-ghost`, `.btn-icon`, etc.
- **Forms:** `.input-base`, `.textarea`, `.select`, `.checkbox`, `.radio`
- **Cards:** `.card`, `.card-header`, `.card-body`, `.card-footer`, `.card-interactive`
- **Alerts:** `.alert`, `.alert-success`, `.alert-error`, `.alert-warning`, `.alert-info`
- **Layout:** `.page-container`, `.content-container`, `.form-container`
- **Typography:** `.text-h1`, `.text-h2`, `.text-body`, `.text-muted`
- **Navigation:** `.navbar`, `.navbar-link`, `.avatar`
- **Utilities:** `.sr-only`, `.focus-ring`, `.hover-lift`, `.line-clamp-*`

Tutte queste classi funzionano correttamente con v4 dopo la riscrittura.

### Colori Personalizzati

Il progetto definisce palette complete per:

- **Primary:** Blue (#2563eb base)
- **Secondary:** Teal (#0d9488 base)
- **Neutral:** Slate (#475569 base)
- **Success:** Green (#16a34a base)
- **Error:** Red (#dc2626 base)
- **Warning:** Amber (#d97706 base)
- **Info:** Sky (#0284c7 base)

Tutte con varianti da 50 a 900.

---

**Data creazione:** 2026-02-09
**Versione Target:** Tailwind CSS v4.0+
