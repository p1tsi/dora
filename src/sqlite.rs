use rusqlite::Connection;
use rusqlite::params;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::consts::{
    ENTITLEMENTS_VALUE_BY_SERVICE_LABEL, INSERT_LIBRARY, INSERT_MACH_SERVICE,
    INSERT_SERVICE_ENTITLEMENT, INSERT_SYMBOL, LIBRARIES_BY_LABEL, MACH_SERVICES_BY_LABEL,
    SERVICE_BY_LABEL, SERVICES_BY_ENTITLEMENT, SERVICES_BY_ENTITLEMENT_AND_SYMBOL,
    SERVICES_BY_LABEL_PATTERN, SERVICES_BY_LIBRARY, SERVICES_BY_SYMBOL, SYMBOLS_BY_LABEL,
};
use crate::macho::*;
use crate::utils::parse_service_plist;

// Function to read SQL queries from a file
// This function takes a file name as input and reads the SQL queries from it
// It returns the queries as a string
fn read_sql_queries_from_file<P: AsRef<Path>>(
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

////////////////////////////////////////////////
///////// SAVE DATA TO SQLITE DATABASE /////////
////////////////////////////////////////////////

// Function that takes the parsed JSON for a plist file and saves it to a SQLite database
fn save_service(
    plist_path: &String,
    json: &JsonValue,
    conn: &rusqlite::Connection,
) -> Result<i64, Box<dyn std::error::Error>> {
    // Extract values from the JSON object
    let label: &str = json.get("Label").and_then(JsonValue::as_str).unwrap_or("");
    let mut path: &str = json
        .get("Program")
        .and_then(JsonValue::as_str)
        .unwrap_or("");

    // If "Program" is not present, try "ProgramArguments"
    if path.is_empty() {
        path = json
            .get("ProgramArguments")
            .and_then(JsonValue::as_array)
            .and_then(|args| args.get(0))
            .and_then(JsonValue::as_str)
            .unwrap_or("");
    }

    // if "plist_path" contains "LaunchAgents" the "run_as_user" is 501 else 0
    let run_as_user: &str = if plist_path.contains("LaunchAgents") {
        "standard" // User ID for the current user
    } else {
        "root" // System service, run as root
    };

    // Get "RunAtLoad" and "KeepAlive" values from JSON that are Bool(true/false)
    let run_at_load: i32 = json
        .get("RunAtLoad")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false) as i32;

    let keep_alive: i32 = json
        .get("KeepAlive")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false) as i32;

    let service_id = insert_and_get_id(
        "service",
        &[
            "label",
            "path",
            "run_as_user",
            "run_at_load",
            "keep_alive",
            "plist_path",
        ],
        &[
            label,
            path,
            run_as_user,
            &run_at_load.to_string(),
            &keep_alive.to_string(),
            plist_path,
        ],
        conn,
    );

    //println!("Inserted service data for label: {}", label);

    service_id
}

// Function that takes service id, JSON formatted entitlements and saves them
// to "entitlement" table in SQLite database and "service_entitlement" table to link entitlements with services
fn save_service_entitlements(
    service_id: i64,
    entitlements: &JsonValue,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Iterate over the entitlements JSON object and insert each entitlement
    if let JsonValue::Object(entitlements_map) = entitlements {
        for (key, value) in entitlements_map {
            // Insert the entitlement into the entitlement table and get its id
            let entitlement_id: i64 = insert_and_get_id("entitlement", &["name"], &[key], conn)?;

            // The value could be a string, a boolean, a number, an array or a dictionary
            let value_str = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Array(arr) => arr
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                JsonValue::Object(obj) => obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join(", "),
                _ => String::from("!!! Not handled !!!"), // Handle other types as needed
            };

            // Insert the service entitlement into the service_entitlement table
            conn.execute(
                INSERT_SERVICE_ENTITLEMENT,
                rusqlite::params![service_id, entitlement_id, value_str.as_str()],
            )?;
        }
    }

    Ok(())
}

// Function to save mach services data to the database
fn save_mach_services(
    service_id: i64,
    json: &JsonValue,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Iterate over the JSON object and insert each mach service
    if let Some(mach_services) = json.get("MachServices") {
        if let JsonValue::Object(services) = mach_services {
            for (name, value) in services {
                let value_str: &str = value.as_str().unwrap_or("");
                conn.execute(
                    INSERT_MACH_SERVICE,
                    rusqlite::params![name, value_str, service_id],
                )?;
            }
        }
    }

    Ok(())
}

// Function that saves to SQLite database the dependencies and
// the relationship between the Mach service and the dependencies
fn save_services_dependencies(
    service_id: i64,
    dependencies: Vec<String>,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Insert each dependency into the mach_service table
    for dep in dependencies {
        // Get dependency name
        let library_name = dep.split('/').last().unwrap_or(&dep).to_string();
        let library_id: i64 =
            insert_and_get_id("library", &["name", "path"], &[&library_name, &dep], conn)?;

        // Insert the relationship between the mach service and the library
        conn.execute(INSERT_LIBRARY, rusqlite::params![service_id, library_id])?;
    }

    Ok(())
}

// Function that saves to SQLite database the binaries and their imported symbols
fn save_service_imported_symbols(
    service_id: i64,
    symbols: Vec<String>,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Insert each symbol into the binary_imported_symbol table
    for symbol in symbols {
        let symbol_id: i64 = insert_and_get_id("symbol", &["name"], &[&symbol], conn)?;

        // Insert the relationship between the service and the symbol
        conn.execute(INSERT_SYMBOL, rusqlite::params![service_id, symbol_id])?;
    }

    Ok(())
}

// Function that takes a Mach-O binary file path and extract all the information from it
// and saves it to the SQLite database
pub fn process_and_save_macho_information(
    binary: &str,
    service_id: i64,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get entitlements for the binary or go on.
    match get_macho_entitlements(binary) {
        Ok(entitlements_json) => {
            save_service_entitlements(service_id, &entitlements_json, &conn)
                .expect("Failed to save service entitlements to database");
        }
        Err(e) => eprintln!("Failed to get entitlements for binary {:?}: {}", binary, e),
    };

    // Get binary external dependencies
    match get_macho_external_dependencies(binary) {
        Ok(dependencies) => {
            // Print the external dependencies
            if !dependencies.is_empty() {
                let _ = save_services_dependencies(service_id, dependencies.clone(), &conn);
            } else {
                println!("No external dependencies found for binary {:?}", binary);
            }
        }
        Err(e) => eprintln!(
            "Failed to get external dependencies for binary {:?}: {}",
            binary, e
        ),
    }

    // Get binary imported symbols
    match get_macho_imported_symbols(&binary) {
        Ok(symbols) => {
            if !symbols.is_empty() {
                let _ = save_service_imported_symbols(service_id, symbols, &conn);
            } else {
                println!("No imported symbols found for binary: {}", binary);
            }
        }
        Err(e) => eprintln!(
            "Failed to get imported symbols for binary {:?}: {}",
            binary, e
        ),
    }

    Ok(())
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
    let res = conn.execute(&insert_sql, rusqlite::params_from_iter(values.iter()))?;
    let id: i64;
    if res == 0 {
        let get_id_sql = format!("SELECT id FROM {} WHERE {} = ?1", table, columns[0]);
        id = conn
            .query_row(&get_id_sql, rusqlite::params![values[0]], |row| row.get(0))
            .expect("Failed to get id from database");
    } else {
        // If the insert was successful, get the last inserted row id
        id = conn.last_insert_rowid();
    }

    Ok(id)
}

pub fn populate_db(sqlite_filename: &String) -> Result<(), Box<dyn std::error::Error>> {
    // Read SQL queries from a file
    let creation_queries = read_sql_queries_from_file("creation_query.sql")
        .expect("Failed to read SQL queries from file");

    let conn = Connection::open(sqlite_filename).expect("Failed to open SQLite database");
    // Execute the SQL queries to create the database
    conn.execute_batch(&creation_queries)
        .expect("Failed to execute SQL queries to create the database");

    println!("Database created successfully at {}", sqlite_filename);

    let launch_paths = [
        //"/Library/LaunchAgents",
        //"/Library/LaunchDaemons",
        "/System/Library/LaunchAgents",
        "/System/Library/LaunchDaemons",
    ];

    // Iterate over launch_paths and process each directory

    launch_paths.iter().for_each(|&launch_path| {
        let paths = std::fs::read_dir(launch_path)
            .expect(format!("Failed to read {} directory", launch_path).as_str());

        paths.for_each(|entry| {
            let path = entry.expect("Failed to read entry").path();

            println!("Processing plist file: {:?}", path);

            match parse_service_plist(&path) {
                Ok(plist_json) => {
                    // Save service data to SQLite database
                    let service_id: i64 =
                        save_service(&path.to_string_lossy().to_string(), &plist_json, &conn)
                            .expect("Failed to save parsed plist data to database");

                    // Save mach services data to SQLite database
                    save_mach_services(service_id, &plist_json, &conn)
                        .expect("Failed to save mach services data to database");

                    // Now analyze the binary
                    // Get the binary path from the JSON object
                    // The binary path can be found in "Program" or "ProgramArguments" fields
                    let binary_path = plist_json
                        .get("Program")
                        .and_then(JsonValue::as_str)
                        .or_else(|| {
                            plist_json
                                .get("ProgramArguments")
                                .and_then(JsonValue::as_array)
                                .and_then(|args| args.get(0))
                                .and_then(JsonValue::as_str)
                        });

                    // Save entitlements for the binary if it exists
                    if let Some(binary) = binary_path {
                        match process_and_save_macho_information(binary, service_id, &conn) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Failed to process Mach-O binary {}: {}", binary, e)
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Failed to parse plist file {:?}: {}", path, e),
            }
        });
    });

    // Iterate over all mach-o binaries under /System/Library/PrivateFrameworks, /usr/bin, /sbin, /usr/sbin
    // and all of its subdirectories
    let folders_to_scan = [
        "/System/Library/PrivateFrameworks",
        "/usr/bin",
        "/sbin",
        "/usr/sbin",
    ];

    folders_to_scan.iter().for_each(|&folder| {
        let entries = std::fs::read_dir(folder)
            .expect(format!("Failed to read {} directory", folder).as_str());

        entries.for_each(|entry| {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();

            if path.is_file() && path.is_macho() {
                // Process the Mach-O binary
                println!("Processing Mach-O binary: {:?}", path);

                let identifier = match get_macho_identifier(path.to_str().unwrap()) {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!(
                            "Failed to get identifier for binary {}: {}",
                            path.display(),
                            e
                        );
                        return;
                    }
                };

                let service_id: i64 = insert_and_get_id(
                    "service",
                    &["label", "path"],
                    &[identifier.as_str(), path.to_str().unwrap()],
                    &conn,
                )
                .expect("Failed to insert service data");

                match process_and_save_macho_information(path.to_str().unwrap(), service_id, &conn)
                {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to process Mach-O binary {}: {}", path.display(), e)
                    }
                }
            }
        });
    });

    // SQLite database connection is automatically closed when it goes out of scope

    Ok(())
}

//////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////

//////////////////////////////////////////////////////////
//////// LOOK FOR SERVICES FROM SQLITE DATABASE //////////
//////////////////////////////////////////////////////////

// Get services from SQLite database that have a sepcified entitlement AND
// a specified symbol
pub fn get_services_by_entitlement_and_symbol(
    db: &String,
    entitlement: &str,
    symbol: &str,
) -> Result<Vec<String>, rusqlite::Error> {
    let conn = match rusqlite::Connection::open(db) {
        Ok(conn) => conn,
        Err(e) => return Err(e),
    };

    let mut stmt = conn.prepare(SERVICES_BY_ENTITLEMENT_AND_SYMBOL)?;
    let result_set = stmt.query_map(
        params![format!("%{}%", entitlement), format!("*{}*", symbol)],
        |row| {
            Ok((
                row.get::<_, String>(0)?, // label
                row.get::<_, String>(1)?, // path
            ))
        },
    )?;

    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path)) => {
                services.push(format!(
                    "<li><strong>Label:</strong> <a href=\"/service?db={db}&label={label}\">{label}</a> ({path})<br>"
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving services by entitlement and symbol: {}", e);
            }
        }
    }

    if services.is_empty() {
        return Ok(vec![format!(
            "<p>No services found with entitlement: {entitlement} and symbol: {symbol}</p>"
        )]);
    }

    let mut html = String::new();
    html.push_str(
        format!(
            "<h2>Found {} services with entitlement: {entitlement} and symbol: {symbol}</h2>",
            services.len()
        )
        .as_str(),
    );
    for service in services {
        html.push_str(&service);
    }

    Ok(vec![html])
}

//

// Get all services from SQLite database having a specific symbol.
// Handle multiple services retrieved by symbol.
pub fn get_services_by_symbol(db: &String, symbol: &str) -> Result<Vec<String>, rusqlite::Error> {
    let conn = match rusqlite::Connection::open(db) {
        Ok(conn) => conn,
        Err(e) => return Err(e),
    };

    let mut stmt = conn.prepare(SERVICES_BY_SYMBOL)?;
    let result_set = stmt.query_map(params![format!("*{}*", symbol)], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
        ))
    })?;
    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path)) => {
                services.push(format!(
                    "<li><strong>Label:</strong> <a href=\"/service?db={db}&label={label}\">{label}</a> ({path})<br>"
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving service by symbol: {}", e);
            }
        }
    }
    if services.is_empty() {
        return Ok(vec![format!(
            "<p>No services found with symbol: {}</p>",
            symbol
        )]);
    }
    let mut html = String::new();
    html.push_str(
        format!(
            "<h2>Found {} services with symbol: {}</h2>",
            services.len(),
            symbol
        )
        .as_str(),
    );
    for service in services {
        html.push_str(&service);
    }

    Ok(vec![html])
}

// Get all services from SQLite database importing a specific library.
// Handle multiple services retrieved by library.
pub fn get_services_by_library(db: &String, library: &str) -> Result<Vec<String>, rusqlite::Error> {
    let conn = match rusqlite::Connection::open(db) {
        Ok(conn) => conn,
        Err(e) => return Err(e),
    };

    let mut stmt = conn.prepare(SERVICES_BY_LIBRARY)?;
    let result_set = stmt.query_map(params![format!("%{}%", library)], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
        ))
    })?;
    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path)) => {
                services.push(format!(
                    "<li><strong>Label:</strong> <a href=\"/service?db={db}&label={label}\">{label}</a> ({path})<br>"
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving service by library: {}", e);
            }
        }
    }
    if services.is_empty() {
        return Ok(vec![format!(
            "<p>No services found with library: {}</p>",
            library
        )]);
    }
    let mut html = String::new();
    html.push_str(
        format!(
            "<h2>Found {} services with library: {}</h2>",
            services.len(),
            library
        )
        .as_str(),
    );
    for service in services {
        html.push_str(&service);
    }
    Ok(vec![html])
}

pub fn get_services_by_entitlement(
    db: &String,
    entitlement: &str,
) -> Result<Vec<String>, rusqlite::Error> {
    let conn = match rusqlite::Connection::open(db) {
        Ok(conn) => conn,
        Err(e) => return Err(e),
    };

    let mut stmt = conn.prepare(SERVICES_BY_ENTITLEMENT)?;
    let result_set = stmt.query_map(params![format!("%{}%", entitlement)], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
        ))
    })?;
    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path)) => {
                services.push(format!(
                    "<li><strong>Label:</strong> <a href=\"/service?db={db}&label={label}\">{label}</a> ({path})<br>"
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving service by entitlement: {}", e);
            }
        }
    }
    if services.is_empty() {
        return Ok(vec![format!(
            "<p>No services found with entitlement: {}</p>",
            entitlement
        )]);
    }
    let mut html = String::new();
    html.push_str(
        format!(
            "<h2>Found {} services with entitlement: {}</h2>",
            services.len(),
            entitlement
        )
        .as_str(),
    );
    for service in services {
        html.push_str(&service);
    }

    Ok(vec![html])
}

// Get service from SQLite database by label case insensitive.
// Handle multiple services retrieved by label.
pub fn get_services_by_label_pattern(
    db: &String,
    label_pattern: &str,
) -> Result<String, rusqlite::Error> {
    let conn = match rusqlite::Connection::open(db) {
        Ok(conn) => conn,
        Err(e) => return Err(e),
    };

    let mut stmt = conn.prepare(SERVICES_BY_LABEL_PATTERN)?;
    let result_set = stmt.query_map(params![format!("*{}*", label_pattern)], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
        ))
    })?;

    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path)) => {
                services.push((label, path));
            }
            Err(e) => {
                eprintln!("Error retrieving service: {}", e);
            }
        }
    }
    if services.is_empty() {
        return Ok(format!(
            "<p>No service found with label: {label_pattern}</p>"
        ));
    }
    let mut html = String::new();
    html.push_str(
        format!(
            "<h2>Found {} services with label pattern: {label_pattern}</h2>",
            services.len()
        )
        .as_str(),
    );
    for (label, path) in services {
        html.push_str(&format!(
            "<li><strong>Service:</strong> <a href=\"/service?db={db}&label={label}\">{label}</a> ({path})</li>"
        ));
    }
    Ok(html)
}

////////////////////////////////////////////////
////////////////////////////////////////////////
////////////////////////////////////////////////

//////////////////////////////////////////////////
//////// LOOK FOR SERVICE INFO BY LABEL //////////
//////////////////////////////////////////////////

// Get all service columns from SQLite database by label
pub fn get_service_by_label(
    conn: &rusqlite::Connection,
    label: &str,
) -> Option<(String, String, String, String, String, String)> {
    let mut stmt = conn.prepare(SERVICE_BY_LABEL).unwrap();

    // Get result set by label considering that some fields can be NULL.
    let result_set = stmt.query_row(params![label], |row| {
        Ok((
            row.get::<_, String>(0)?,                                // label
            row.get::<_, String>(1)?,                                // path
            row.get::<_, String>(2).unwrap_or(String::from("NULL")), // run_as_user
            row.get::<_, String>(3).unwrap_or(String::from("NULL")), // run_at_load
            row.get::<_, String>(4).unwrap_or(String::from("NULL")), // keep_alive
            row.get::<_, String>(5).unwrap_or(String::from("NULL")), // plist_path
        ))
    });

    match result_set {
        Ok((label, path, run_as_user, run_at_load, keep_alive, plist_path)) => Some((
            label,
            path,
            run_as_user,
            run_at_load,
            keep_alive,
            plist_path,
        )),
        Err(_) => None,
    }
}

pub fn get_mach_service_by_label(conn: &rusqlite::Connection, label: &str) -> Option<Vec<String>> {
    let mut stmt = conn.prepare(MACH_SERVICES_BY_LABEL).unwrap();

    // Get result set by label considering that some fields can be NULL.
    let result_set = stmt.query_map(params![label], |row| {
        Ok(row.get::<_, String>(0).unwrap_or(String::from("NULL")))
    });

    let mut mach_services = Vec::new();
    match result_set {
        Ok(rows) => {
            for row in rows {
                match row {
                    Ok(service) => mach_services.push(service),
                    Err(_) => return None,
                }
            }
            if mach_services.is_empty() {
                None
            } else {
                Some(mach_services)
            }
        }
        Err(_) => None,
    }
}

// Get entitlements values by service label
pub fn get_entitlements_value_by_service_label(
    conn: &rusqlite::Connection,
    service_label: &str,
) -> Option<HashMap<String, String>> {
    let mut stmt = conn.prepare(ENTITLEMENTS_VALUE_BY_SERVICE_LABEL).unwrap();

    // Get result set by label considering that some fields can be NULL.
    let result_set = stmt.query_map(params![service_label], |row| {
        Ok((
            row.get::<_, String>(0)?, // entitlement_name
            row.get::<_, String>(1)?, // entitlement_value
        ))
    });

    let mut entitlements = HashMap::new();
    match result_set {
        Ok(rows) => {
            for row in rows {
                match row {
                    Ok((name, value)) => {
                        entitlements.insert(name, value);
                    }
                    Err(_) => return None,
                }
            }
            if entitlements.is_empty() {
                None
            } else {
                Some(entitlements)
            }
        }
        Err(_) => None,
    }
}

// Get libraries by label from SQLite database
pub fn get_libraries_by_label(
    conn: &rusqlite::Connection,
    label: &str,
) -> Option<Vec<(String, String)>> {
    let mut stmt = conn.prepare(LIBRARIES_BY_LABEL).unwrap();

    // Get result set by label considering that some fields can be NULL.
    let result_set = stmt.query_map(params![label], |row| {
        Ok((
            row.get::<_, String>(0)?, // library name
            row.get::<_, String>(1)?, // library path
        ))
    });

    let mut libraries = Vec::new();
    match result_set {
        Ok(rows) => {
            for row in rows {
                match row {
                    Ok((name, path)) => libraries.push((name, path)),
                    Err(_) => return None,
                }
            }
            if libraries.is_empty() {
                None
            } else {
                Some(libraries)
            }
        }
        Err(_) => None,
    }
}

// Get symbols by label from SQLite database
pub fn get_symbols_by_label(conn: &rusqlite::Connection, label: &str) -> Option<Vec<String>> {
    let mut stmt = conn.prepare(SYMBOLS_BY_LABEL).unwrap();
    let result_set: Vec<String> = stmt
        .query_map(params![label], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    if result_set.is_empty() {
        None
    } else {
        Some(result_set)
    }
}

////////////////////////////////////////////////
////////////////////////////////////////////////
////////////////////////////////////////////////
