use plist::Value;
use serde_json::Value as JsonValue;
use std::fs::File;
use std::path::Path;

// Create SQLite database file name
pub fn generate_sqlite_filename() -> String {
    // Create sqlite db file name.
    // The file name format is "dora_<product_name>_<product_version>_<build_version>.sqlite"

    // Get product name
    let product_name: String = std::process::Command::new("sw_vers")
        .arg("-productName")
        .output()
        .expect("Failed to get product name")
        .stdout
        .into_iter()
        .map(|b| b as char)
        .collect::<String>()
        .trim()
        .to_string();

    // Get product version
    let product_version: String = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .expect("Failed to get product version")
        .stdout
        .into_iter()
        .map(|b| b as char)
        .collect::<String>()
        .trim()
        .to_string();

    // Get build version
    let build_version: String = std::process::Command::new("sw_vers")
        .arg("-buildVersion")
        .output()
        .expect("Failed to get build version")
        .stdout
        .into_iter()
        .map(|b| b as char)
        .collect::<String>()
        .trim()
        .to_string();

    // Create the SQLite database file name
    let sqlite_filename = format!(
        "dora_{}_{}_{}.sqlite",
        product_name, product_version, build_version
    );

    sqlite_filename
}

// Function that takes a path as input and parse the plist file
pub fn parse_service_plist<P: AsRef<Path>>(
    path: P,
) -> Result<JsonValue, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let plist_value = Value::from_reader(file)?;
    let json = serde_json::to_value(plist_value)?;

    Ok(json)
}

// Get avaliable SQLite databases looking for ".sqlite" files
pub fn get_available_databases() -> Vec<String> {
    let mut databases = Vec::new();
    let paths = std::fs::read_dir(".").expect("Failed to read current directory");

    for entry in paths {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                databases.push(name.to_string());
            }
        }
    }

    databases
}

// Function that validates db param
// Make sure db is not empty, starts with "dora_", ends with ".sqlite" and not contains "/" character.
pub fn is_valid_db(db: &String) -> bool {
    !db.is_empty() && db.starts_with("dora_") && db.ends_with(".sqlite") && !db.contains('/')
}
