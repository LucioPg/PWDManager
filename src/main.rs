use gui_launcher::check_paths;

fn create_table() -> Result<(), Box<dyn std::error::Error>>{
    let query  = "\
    CREATE TABLE IF NOT EXISTS users (username Text, password Text);";
    let conn = sqlite::open("database.db")?;
    let _ = conn.execute(query)?;
    Ok(())

}

fn main() {
    println!("Creating database and table...");
    let db_creation: Result<(), Box<dyn std::error::Error>> = create_table();
    match  db_creation {
        Ok(()) => println!("Database created!"),
        Err(e) => println!("An error occurred while creating the database: {e}")
    };
    println!("checking paths");
    let existent_paths = check_paths();
    println!("paths: {}", existent_paths);
}
