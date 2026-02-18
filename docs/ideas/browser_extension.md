Per creare un'estensione dedicata che comunichi con la tua app Dioxus 0.7 Desktop, devi implementare un'architettura
basata su Native Messaging. Questo è lo standard di sicurezza che permette ai browser (Chrome, Firefox, Edge) di
scambiare messaggi con un'applicazione locale installata sul PC.
Chrome for Developers
Chrome for Developers
+3
Ecco i tre componenti necessari per farli parlare tra loro:

1. L'Estensione (Frontend Browser)
   Puoi usare il pacchetto dx-ext (Dioxus Browser Extension Builder) per scrivere l'estensione direttamente in Rust.
   [Crates.io](https://crates.io/crates/dioxus-browser-extension-builder#:~:text=Overview,configuration%20via%20a%20TOML%20file)
   Crates.io
   Funzionamento: L'estensione si collega alla tua app usando chrome.runtime.connectNative("nome_tua_app").
   Azione: Quando l'utente clicca su un campo di login nel browser, l'estensione invia un messaggio JSON alla tua app
   desktop chiedendo le credenziali per quel dominio.
   textslashplain.com
   textslashplain.com
2. Il "Native Messaging Host" (Il Ponte)
   Affinché il browser accetti di parlare con la tua app, devi registrare un file manifest JSON in una cartella
   specifica del sistema operativo (es. nel Registro di Windows o in /Library/... su macOS).
   Stack Overflow
   Stack Overflow
   Questo file dichiara al browser il percorso dell'eseguibile della tua app Dioxus e quali estensioni sono autorizzate
   a contattarla.
   Puoi usare la crate native_messaging per gestire l'installazione automatica di questo manifest e la comunicazione
   sicura.
   [Crates.io](https://crates.io/crates/native_messaging#:~:text=native_messaging%20%2D%20crates.io:%20Rust,is%20reserved%20for%20framed%20messages.)

3. L'App Desktop (Backend Dioxus)
   La tua app Dioxus deve essere in grado di leggere messaggi da stdin e scrivere su stdout seguendo un protocollo
   preciso (un prefisso di 4 byte per la lunghezza seguito dal JSON).
   Docs.rs
   Docs.rs
   ```rust
   // Esempio concettuale di ricezione messaggio nell'app desktop
   use native_messaging::read_message;

    // In un thread separato della tua app Dioxus
    loop {
    if let Ok(msg) = read_message::<MyRequest>() {
    // Cerca la password nel database locale e rispondi
    }
    }

    ```

Usa il codice con cautela.

Perché questa è la scelta migliore?
Sicurezza: Solo le estensioni autorizzate possono accedere ai dati.
Automazione: Puoi intercettare il dominio della pagina aperta (tabs.query) e proporre all'utente solo le password
pertinenti, proprio come fa Bitwarden o 1Password.
Stack Overflow
Stack Overflow
+1
Vuoi che ti mostri come configurare il file manifest JSON necessario per far sì che Chrome o Firefox riconoscano la tua
app Dioxus?
Le risposte dell'AI potrebbero contenere errori. Scopri di più