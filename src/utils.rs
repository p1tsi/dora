use plist::Value;
use serde_json::Value as JsonValue;
use std::fs::File;
use std::path::Path;

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
