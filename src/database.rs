use serde_json::Value as JsonValue;

use crate::utils::parse_service_plist;
use crate::sqlite::{create_db, read_sql_queries_from_file};
use crate::sqlite::insert_and_get_id;



// Get macho binary entitlements launching "codesign" command
fn get_macho_entitlements(binary_path: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
    // Execute the following command to get JSON formatted entitlements from a Mach-O binary
    // "codesign --display --entitlements :- <binary_path> | plutil -convert json -o - -"
    let codesign_output = std::process::Command::new("codesign")
        .args(["-d", "--entitlements", ":-", binary_path])
        .output()
        .expect("Failed to execute codesign");

    if !codesign_output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get entitlements for binary: {}", binary_path),
        )));
    }

    // Check if the output is empty
    if codesign_output.stdout.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No entitlements found for binary: {}", binary_path),
        )));
    }

    let mut plutil = std::process::Command::new("plutil")
        .args(["-convert", "json", "-o", "-", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to execute plutil");

    {
        use std::io::Write;
        let stdin = plutil.stdin.as_mut().expect("Failed to open plutil stdin");
        stdin
            .write_all(&codesign_output.stdout)
            .expect("Failed to write to plutil");
    }

    let output = plutil
        .wait_with_output()
        .expect("Failed to read plutil output");

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to convert entitlements to JSON for binary: {}",
                binary_path
            ),
        )));
    }

    let entitlements_json: JsonValue =
        serde_json::from_slice(&output.stdout).expect("Failed to parse entitlements JSON");

    Ok(entitlements_json)
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
                "INSERT OR IGNORE INTO service_entitlement (service_id, entitlement_id, value) VALUES (?1, ?2, ?3)",
                rusqlite::params![service_id, entitlement_id, value_str.as_str()],
            )?;
        }
    }

    Ok(())
}


// Function that takes the parsed JSON for a plist file and saves it to a SQLite database
fn save_service(
    plist_path: &String,
    json: &JsonValue,
    conn: &rusqlite::Connection,
) -> Result<i64, Box<dyn std::error::Error>> {
    // Insert the JSON data into service table
    let sql: &'static str = "INSERT OR IGNORE INTO service (label, path, run_as_user, run_at_load, keep_alive, plist_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

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
    let run_as_user: i32 = if plist_path.contains("LaunchAgents") {
        501 // User ID for LaunchAgents
    } else {
        0 // User ID for LaunchDaemons
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

    // Execute the SQL statement to insert the service data
    conn.execute(
        sql,
        rusqlite::params![
            label,
            path,
            run_as_user,
            run_at_load,
            keep_alive,
            plist_path
        ],
    )?;

    //println!("Inserted service data for label: {}", label);

    Ok(conn.last_insert_rowid())
}

// Function to save mach services data to the database
fn save_mach_services(
    service_id: i64,
    json: &JsonValue,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Insert the JSON data into mach_service table
    let sql: &'static str =
        "INSERT OR IGNORE INTO mach_service (name, value, service_id) VALUES (?1, ?2, ?3)";

    // Iterate over the JSON object and insert each mach service
    if let Some(mach_services) = json.get("MachServices") {
        if let JsonValue::Object(services) = mach_services {
            for (name, value) in services {
                let value_str: &str = value.as_str().unwrap_or("");
                conn.execute(sql, rusqlite::params![name, value_str, service_id])?;
            }
        }
    }

    Ok(())
}


// Function that extracts external dependencies from a Mach-O binary
// launching "otool -L <binary_path>" command
fn get_external_dependencies(binary_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Execute the otool command to get external dependencies
    let output = std::process::Command::new("otool")
        .args(["-L", binary_path])
        .output()
        .expect("Failed to execute otool");

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to get external dependencies for binary: {}",
                binary_path
            ),
        )));
    }

    // Parse the output and extract the dependencies
    let dependencies: Vec<String> = String::from_utf8(output.stdout)
        .expect("Failed to parse otool output")
        .lines()
        .skip(1) // Skip the first line which is the binary name
        .map(|line| line.split_whitespace().next().unwrap_or("").to_string())
        .filter(|dep| !dep.is_empty())
        .collect();

    Ok(dependencies)
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
        conn.execute(
            "INSERT OR IGNORE INTO service_library (service_id, library_id) VALUES (?1, ?2)",
            rusqlite::params![service_id, library_id],
        )?;
    }

    Ok(())
}

// Function that extracts binary imported symbols
// launching "nm -u <binary_path>" command
fn get_binary_imported_symbols(
    binary_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Execute the nm command to get imported symbols
    let output = std::process::Command::new("nm")
        .args(["-u", "--arch=arm64e", binary_path])
        .output()
        .expect("Failed to execute nm");

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get imported symbols for binary: {}", binary_path),
        )));
    }

    // Parse the output and extract the symbols
    let symbols: Vec<String> = String::from_utf8(output.stdout)
        .expect("Failed to parse nm output")
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(symbols)
}

// Function that saves to SQLite database the binaries and their imported symbols
fn save_binary_imported_symbols(
    service_id: i64,
    symbols: Vec<String>,
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    // Insert each symbol into the binary_imported_symbol table
    for symbol in symbols {
        let symbol_id: i64 = insert_and_get_id("symbol", &["name"], &[&symbol], conn)?;

        // Insert the relationship between the service and the symbol
        conn.execute(
            "INSERT OR IGNORE INTO service_symbol (service_id, symbol_id) VALUES (?1, ?2)",
            rusqlite::params![service_id, symbol_id],
        )?;
    }

    Ok(())
}

pub fn populate_db(sqlite_filename: &String) -> Result<(), Box<dyn std::error::Error>> {
    // Read SQL queries from a file
    let creation_queries = read_sql_queries_from_file("creation_query.sql")
        .expect("Failed to read SQL queries from file");

    create_db(&sqlite_filename, &creation_queries)
        .expect("Failed to create database from SQL queries");

    // Open the SQLite database connection
    let conn =
        rusqlite::Connection::open(&sqlite_filename).expect("Failed to open SQLite database");

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
                        // Get entitlements for the binary or go on.
                        let _ = match get_macho_entitlements(binary) {
                            Ok(entitlements_json) => {
                                save_service_entitlements(service_id, &entitlements_json, &conn)
                                    .expect("Failed to save service entitlements to database");
                            }
                            Err(e) => eprintln!(
                                "Failed to get entitlements for binary {:?}: {}",
                                binary, e
                            ),
                        };

                        // Get binary external dependencies
                        match get_external_dependencies(binary) {
                            Ok(dependencies) => {
                                // Print the external dependencies
                                if !dependencies.is_empty() {
                                    let _ = save_services_dependencies(
                                        service_id,
                                        dependencies.clone(),
                                        &conn,
                                    );
                                } else {
                                    println!(
                                        "No external dependencies found for binary {:?}",
                                        binary
                                    );
                                }
                            }
                            Err(e) => eprintln!(
                                "Failed to get external dependencies for binary {:?}: {}",
                                binary, e
                            ),
                        }

                        // Get binary imported symbols
                        match get_binary_imported_symbols(&binary) {
                            Ok(symbols) => {
                                if !symbols.is_empty() {
                                    let _ =
                                        save_binary_imported_symbols(service_id, symbols, &conn);
                                } else {
                                    println!("No imported symbols found for binary: {}", binary);
                                }
                            }
                            Err(e) => eprintln!(
                                "Failed to get imported symbols for binary {:?}: {}",
                                binary, e
                            ),
                        }
                    }
                }
                Err(e) => eprintln!("Failed to parse plist file {:?}: {}", path, e),
            }
        });
    });

    // Close the SQLite database connection
    conn.close()
        .expect("Failed to close SQLite database connection");

    Ok(())
}