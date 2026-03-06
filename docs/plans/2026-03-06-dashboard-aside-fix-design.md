# Design: Fix Aside Collassato - Centraggio Badge

**Data:** 2026-03-06
**Stato:** Approvato
**Priorità:** Media (bug fix UI)

## Problema

Nella dashboard, l'aside delle statistiche quando è collassato presenta un problema di frontend design:
- Il div che contiene la lettera delle statistiche (badge) non occupa tutto lo spazio disponibile
- Il badge appare "allungato" o non correttamente proporzionato
- La lettera non è centrata correttamente nel badge

### Analisi Tecnica

L'aside collassato ha una larghezza di 52px (`.pwd-stats-aside--collapsed`). Il badge misura 28px x 28px (`.pwd-stats-aside__badge`). Lo spazio effettivo disponibile è ridotto da:
- Toggle button: 14px (larghezza)
- Item padding: `0.35rem 0.25rem` (circa 5-6px per lato)

Questo crea un divario tra il badge e i bordi dell'aside, facendolo apparire non centrato.

## Soluzione Proposta

**Approccio 1: Item centrato con flexbox (Raccomandato)**

Rendere l'item container flex centrato quando l'aside è collassato, rimuovendo il padding laterale.

### Modifiche CSS Richieste

File: `assets/input_main.css`

Aggiungere una nuova regola per centrare l'item quando l'aside è collassato:

```css
/* Centra l'item nell'aside collassato */
.pwd-stats-aside--collapsed .pwd-stats-aside__item {
    justify-content: center;
    padding-left: 0;
    padding-right: 0;
}
```

Questa regola:
1. `justify-content: center` - Centra il badge orizzontalmente nel container
2. `padding-left: 0` e `padding-right: 0` - Rimuove il padding laterale per massimizzare lo spazio

## Implementazione

### File da Modificare

1. **`assets/input_main.css`**
   - Aggiungere la regola CSS per l'item centrato nell'aside collassato
   - Posizionare dopo la regola `.pwd-stats-aside__item` (riga 1452)

### Nessuna Modifica Richiesta

- **`src/components/globals/stats_aside.rs`** - Nessuna modifica necessaria (il componente RSX è corretto)
- **`src/components/features/dashboard.rs`** - Nessuna modifica necessaria

## Testing

### Test Cases

1. **Aside collassato:**
   - Verificare che il badge sia perfettamente centrato orizzontalmente
   - Verificare che il badge mantenga la forma quadrata (28px x 28px)
   - Verificare che la lettera sia centrata all'interno del badge

2. **Aside espanso:**
   - Verificare che il layout non sia influenzato dalle modifiche
   - Verificare che badge, valore e label siano allineati correttamente

3. **Transizione:**
   - Verificare che l'animazione tra collassato/espanso sia fluida
   - Verificare che non ci siano salti o glitch visivi

### Border Cases

- Statistiche con conteggi a più cifre (es. 100+ password)
- Font sizes diversi (accessibility zoom)

## Alternative Considerate

### Approccio 2: Badge più grande (Scartato)
Aumentare le dimensioni del badge a 32px.
**Scarto perché:** Richiederebbe aggiustamenti allo spacing quando l'aside è espanso.

### Approccio 3: Grid layout (Scartato)
Usare CSS Grid per il container.
**Scarto perché:** Cambio di paradigma layout che potrebbe introdurre side effects.

## Impatto

- **Breaking Changes:** Nessuno
- **Compatibilità:** Completamente retrocompatibile
- **Performance:** Nessun impatto (solo CSS)
- **Accessibilità:** Migliora l'aspetto visivo senza influire sulla navigazione

## Success Criteria

1. ✅ Il badge è perfettamente centrato nell'aside collassato
2. ✅ Il badge mantiene la forma quadrata
3. ✅ La lettera è centrata all'interno del badge
4. ✅ L'aside espanso funziona come prima
5. ✅ L'animazione di transizione è fluida
