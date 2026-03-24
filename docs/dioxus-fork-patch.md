# Patch Dioxus Fork - `[patch.crates-io]` in Cargo.toml

## Perché serve

PWDManager usa **dioxus 0.7.3** come framework UI. Il CLI di dioxus (`dx`) installato da crates.io è la versione **0.7.3 stabilizzata**, che usa internamente **tauri-bundler v2** per creare l'installer Windows.

Noi usiamo un **fork personale** di dioxus ([github.com/LucioPg/dioxus](https://github.com/LucioPg/dioxus), branch `main`, commit `7289256`) che ha rimosso tauri-bundler e implementato un **native bundler** con un sistema NSIS completamente nuovo.

Se la **libreria** dioxus (quella linkata nel codice Rust) usa crates.io `0.7.3` ma il **CLI** `dx` viene dal fork, si verifica un **version mismatch**: il CLI genera asseti/manifesti con formati diversi da quelli che la libreria si aspetta, causando errori di compilazione a runtime.

La sezione `[patch.crates-io]` risolve questo problema dicendo a Cargo di sostituire **tutti** i pacchetti dioxus di crates.io con le versioni locali del fork.

## Come funziona

```toml
[patch.crates-io]
dioxus = { path = "../../../../dioxus/packages/dioxus" }
dioxus-core = { path = "../../../../dioxus/packages/core" }
dioxus-core-macro = { path = "../../../../dioxus/packages/core-macro" }
dioxus-core-types = { path = "../../../../dioxus/packages/core-types" }
dioxus-desktop = { path = "../../../../dioxus/packages/desktop" }
dioxus-html = { path = "../../../../dioxus/packages/html" }
dioxus-html-internal-macro = { path = "../../../../dioxus/packages/html-internal-macro" }
dioxus-interpreter-js = { path = "../../../../dioxus/packages/interpreter" }
dioxus-rsx = { path = "../../../../dioxus/packages/rsx" }
dioxus-router = { path = "../../../../dioxus/packages/router" }
dioxus-signals = { path = "../../../../dioxus/packages/signals" }
dioxus-hooks = { path = "../../../../dioxus/packages/hooks" }
```

### Risoluzione dei percorsi

Il worktree si trova in:
```
C:\Users\Lucio\RustroverProjects\PWDManager\.claude\worktrees\keyring-sqlcipher
```

Risalendo di 4 livelli (`../../../../`):
```
C:\Users\Lucio\RustroverProjects\dioxus\
```

Questo è il fork locale dove ogni pacchetto si trova sotto `packages/`.

### Nota: `dioxus-interpreter-js`

La directory nel fork si chiama `packages/interpreter`, ma il nome del crate è `dioxus-interpreter-js`. Il mapping è corretto così come scritto.

## Pacchetti patchati

| Crate (crates.io) | Path locale nel fork |
|---|---|
| `dioxus` | `packages/dioxus` |
| `dioxus-core` | `packages/core` |
| `dioxus-core-macro` | `packages/core-macro` |
| `dioxus-core-types` | `packages/core-types` |
| `dioxus-desktop` | `packages/desktop` |
| `dioxus-html` | `packages/html` |
| `dioxus-html-internal-macro` | `packages/html-internal-macro` |
| `dioxus-interpreter-js` | `packages/interpreter` |
| `dioxus-rsx` | `packages/rsx` |
| `dioxus-router` | `packages/router` |
| `dioxus-signals` | `packages/signals` |
| `dioxus-hooks` | `packages/hooks` |

## Prerequisiti

1. Il fork deve essere clonato in `C:\Users\Lucio\RustroverProjects\dioxus\`
2. Il branch `main` deve essere aggiornato (commit `7289256` o superiore)
3. Il dioxus-cli deve essere installato dal fork, non da crates.io:
   ```bash
   cargo install --path C:\Users\Lucio\RustroverProjects\dioxus\packages\cli
   ```

## Quando rimuovere il patch

Quando il fork verrà mergiato upstream e pubblicato una nuova versione su crates.io (es. `0.8.0` o `0.7.4`), basterà:

1. Aggiornare `Cargo.toml` alla nuova versione
2. Rimuovere l'intera sezione `[patch.crates-io]`
3. Reinstallare `dx` da crates.io: `cargo install dioxus-cli`
4. Aggiornare `installer/custom-installer.nsi` per eventuali cambiamenti nel template NSIS del nuovo bundler
