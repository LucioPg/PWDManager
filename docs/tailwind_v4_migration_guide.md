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

Assicurati che il comando di build di Tailwind funzioni ancora:

```bash
# Verifica il comando nel tuo setup
npx tailwindcss -i ./assets/input_main.css -o ./assets/tailwind.css
```

---

## Checklist di Coerenza (Check Manuale Rapido)

Per essere "sicuro al 100%", controlla questi tre punti nel tuo codice:

### ✓ 1. Rimosso il vecchio file di configurazione

Hai eliminato (o svuotato) `tailwind.config.js`?
- In v4 non è più necessario, a meno che tu non voglia mantenere una compatibilità ibrida.

### ✓ 2. Direttive CSS aggiornate

Il tuo file CSS inizia con `@import "tailwindcss";` invece delle vecchie tre righe?

```css
/* CORRETTO (v4) */
@import "tailwindcss";

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
│   ├── tailwind.css      # Output generato
│   └── input.css         # Sorgente Tailwind v3 (da sostituire)
```

1. Il file `input_main.css` è il punto chiave per la migrazione
2. Verifica che le classi personalizzate nel file funzionino ancora con v4
3. Controlla che non ci siano conflitti con le utility ridenominate

---

**Data creazione:** 2026-02-09
**Versione Target:** Tailwind CSS v4.0+
