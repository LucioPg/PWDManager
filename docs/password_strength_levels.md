# Password Strength Levels

Documentazione dei livelli di forza delle password in PWDManager.

## Overview

Il sistema di valutazione delle password assegna uno **score da 0 a 100** basato su più fattori, che viene poi convertito in un livello di forza (`PasswordStrength`).

## Livelli Disponibili

| Livello | Score Range | Descrizione |
|---------|-------------|-------------|
| **MEDIUM** | 50 - 69 | Password accettabile, ma migliorabile |
| **STRONG** | 70 - 84 | Password forte, adeguata per la maggior parte degli usi |
| **EPIC** | 85 - 95 | Password molto forte, alta sicurezza |
| **GOD** | 96 - 100 | Password eccellente, massima sicurezza |

---

## Sistema di Scoring

### Punti Base

#### Lunghezza (max 20 punti)
- **0.5 punti** per carattere
- **Massimo 20 punti** (40+ caratteri)

```rust
bonus = (lunghezza * 0.5).min(20.0)
```

#### Varietà Caratteri (max 60 punti)
- **15 punti** per ogni categoria presente:
  - ✅ Maiuscole (A-Z)
  - ✅ Minuscole (a-z)
  - ✅ Numeri (0-9)
  - ✅ Speciali (!@#$%^&*...)

```rust
variety_count = categorie_presente * 15
// Max: 4 categorie * 15 = 60 punti
```

### Bonus

| Bonus | Punti | Condizione |
|-------|-------|------------|
| Extra lunghezza | **+10** | > 16 caratteri |
| Extra lunghezza | **+5** | > 12 caratteri (e ≤ 16) |
| Speciali multipli | **+5** | 2+ caratteri speciali |
| Alta entropia | **+10** | 16+ caratteri unici |
| Alta entropia | **+5** | 12+ caratteri unici (e < 16) |

### Penalità

- **-10 punti** per ogni errore di validazione (reason)

## Requisiti per Livello

### MEDIUM (50-69 punti)

**Requisiti minimi:**
- Almeno 8 caratteri
- Almeno 3-4 categorie di caratteri
- Nessuna errori critici

**Esempio:**
```
MyPass123!
```
- Lunghezza: 10 → 5 punti
- Varietà: 4 categorie → 60 punti
- **Totale: 65 punti → MEDIUM**

---

### STRONG (70-84 punti)

**Requisiti minimi:**
- Almeno 12+ caratteri
- Tutte e 4 le categorie di caratteri
- Buona varietà (12+ caratteri unici)

**Esempio:**
```
VeryStrongPassword123!@#
```
- Lunghezza: 25 → 12.5 → 12 punti
- Varietà: 4 categorie → 60 punti
- Bonus >16 → +10
- **Totale: 82 punti → STRONG**

---

### EPIC (85-95 punti)

**Requisiti:**
- 14-16+ caratteri
- Tutte e 4 le categorie
- 2+ caratteri speciali
- 12+ caratteri unici (alta entropia)
- Nessun errore di validazione

**Esempio:**
```
ThisIsAVeryStrongP@ssw0rd!2024
```
- Lunghezza: 29 → 14.5 → 14 punti
- Varietà: 4 categorie → 60 punti
- Bonus >16 → +10
- Speciali multipli → +5
- **Totale: 89 punti → EPIC**

---

### GOD (96-100 punti)

**Requisiti:**
- **17+ caratteri** (per il bonus completo)
- **Tutte e 4 le categorie**: maiuscole, minuscole, numeri, speciali
- **2+ caratteri speciali**
- **16+ caratteri unici** (alta entropia)
- **Nessun errore di validazione**:
  - ❌ Non nella blacklist (top 10,000 password comuni)
  - ❌ No pattern ripetitivi (aaa, 111, !!!)
  - ❌ No sequenze (abcd, 1234, dcba, 4321)

**Esempio perfetto:**
```
ThisIsAVeryStrongP@ssw0rd!2024#XyZ
```

**Analisi dettagliata:**
| Criterio | Valore | Punti |
|----------|--------|-------|
| Lunghezza | 33 caratteri | 20 (max) |
| Varietà | 4 categorie | 60 (max) |
| Bonus >16 | Sì | +10 |
| Speciali multipli | 4+ (@!#) | +5 |
| Caratteri unici | 20+ | +10 |
| Penalità | 0 | 0 |
| **TOTALE** | | **105 → capped a 100** → **GOD** |

---

## Regole di Validazione

### 1. Blacklist
- Controlla le top **10,000 password più comuni**
- Se trovata → `-10 punti` + reason

### 2. Lunghezza Minima
- Minimo **8 caratteri**
- Se più corta → `-10 punti` + reason

### 3. Varietà Caratteri
- Deve contenere **tutte e 4 le categorie**
- Se mancante → `-10 punti` + reason

### 4. Pattern Analysis
- **No ripetizioni**: 3+ caratteri uguali consecutivi (aaa, 111)
- **No sequenze**: 4+ caratteri sequenziali (abcd, 1234, dcba, 4321)
- Se trovati → `-10 punti` + reason

---

## Conversione Score → Strength

```rust
pub fn get_strength(score: Option<i64>) -> PasswordStrength {
    match score {
        Some(s) if s > 95 => PasswordStrength::GOD,      // 96-100
        Some(s) if s >= 85 => PasswordStrength::EPIC,    // 85-95
        Some(s) if s >= 70 => PasswordStrength::STRONG,  // 70-84
        Some(s) if s >= 50 => PasswordStrength::MEDIUM,  // 50-69
        Some(_) => PasswordStrength::WEAK,               // 0-49
        None => PasswordStrength::NotEvaluated,
    }
}
```

## Implementazione

**File:** `src/backend/strength_utils.rs`

La funzione principale è `evaluate_password_strength()` che:
1. Esegue le 4 sezioni di validazione
2. Calcola lo score basandosi sui criteri sopra
3. Applica penalità per eventuali errori
4. Ritorna una `PasswordEvaluation` con score e reasons
