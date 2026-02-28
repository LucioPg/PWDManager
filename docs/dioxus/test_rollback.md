In SQLx con SQLite (WAL mode), il test del rollback con un pool di connessioni richiede attenzione alla gestione del
ciclo di vita della transazione, poiché SQLx utilizza il pattern RAII: se l'oggetto Transaction esce dallo scope senza
un commit esplicito, viene eseguito automaticamente un rollback.

Ecco come procedere per testare correttamente il rollback in Rust:

1. Sfruttare il Rollback Automatico (Pattern RAII)
   In SQLx, una Transaction esegue il rollback al momento della sua distruzione (drop) se non è stato chiamato
   .commit(). Questo è ideale per testare il fallimento di un'operazione.
   ```rust
   #[tokio::test]
   async fn test_rollback_on_failure() {
   let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

   // 1. Inizia la transazione
   let mut tx = pool.begin().await.unwrap();

   // 2. Esegui un'operazione
   sqlx::query("INSERT INTO users (name) VALUES (?)")
   .bind("Test User")
   .execute(&mut *tx) // Usa il riferimento mutabile alla transazione
   .await.unwrap();

   // 3. Simula un errore o semplicemente lascia che 'tx' esca dallo scope
   drop(tx); // Rollback automatico qui

   // 4. Verifica con una NUOVA connessione dal pool
   let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
   .fetch_one(&pool)
   .await.unwrap();

   assert_eq!(row.0, 0); // Il record non deve esistere
   }
   ```

2. Testare il Rollback Esplicito
   Se vuoi testare logiche di errore che chiamano esplicitamente .rollback(), devi assicurarti che il test verifichi lo
   stato del DB dopo che il metodo è stato completato.
   Crates.io
   Crates.io
   ```rust
   let mut tx = pool.begin().await?;
   // ... operazioni ...
   tx.rollback().await?; // Rollback manuale
   ```

3. Isolamento con #[sqlx::test]
   Per evitare che i test interferiscano tra loro (specialmente con file fisici in modalità WAL), usa la
   macro [sqlx::test]. Questa macro crea un database isolato per ogni test, gestendo automaticamente le migrazioni.
 