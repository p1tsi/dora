use rusqlite::Connection;
use std::fs::File;
use std::io::Read;
use std::path::Path;

// Function to create a SQLite database from its file name and SQL queries
// This function takes a filename and a string containing SQL queries
// It creates a SQLite database file and executes the SQL queries to set it up
pub fn create_db(filename: &String, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(filename)?;

    // Execute the SQL queries to create the database
    conn.execute_batch(sql)?;

    println!("Database created successfully at {}", filename);

    Ok(())
}

// Function to read SQL queries from a file
// This function takes a file name as input and reads the SQL queries from it
// It returns the queries as a string
pub fn read_sql_queries_from_file<P: AsRef<Path>>(
    file_path: P,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(&file_path)?;
    let mut sql = String::new();

    file.read_to_string(&mut sql)?;
    println!(
        "Read SQL queries from file: {}",
        file_path.as_ref().display()
    );

    Ok(sql)
}

// Insert new item into column(s) and retrieve its id
pub fn insert_and_get_id(
    table: &str,
    columns: &[&str],
    values: &[&str],
    conn: &rusqlite::Connection,
) -> Result<i64, Box<dyn std::error::Error>> {
    // Construct the SQL query dynamically based on the table and columns
    let placeholders: String = (1..=columns.len())
        .map(|i| format!("?{}", i))
        .collect::<Vec<String>>()
        .join(", ");
    let insert_sql = format!(
        "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
        table,
        columns.join(", "),
        placeholders
    );

    // Execute the SQL statement to insert the data
    conn.execute(&insert_sql, rusqlite::params_from_iter(values.iter()))?;

    let get_id_sql = format!("SELECT id FROM {} WHERE {} = ?1", table, columns[0]);
    let id: i64 = conn
        .query_row(&get_id_sql, rusqlite::params![values[0]], |row| row.get(0))
        .expect("Failed to get id from database");

    Ok(id)
}
