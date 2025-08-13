use serde_json::Value as JsonValue;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

// Get Identifier for a Mach-O binary
// launching "codesign -dv <binary_path> 2>&1 | grep '^Identifier' | cut -d= -f2"
pub fn get_macho_identifier(binary_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Execute the codesign command to get the identifier
    let output = std::process::Command::new("codesign")
        .args(["-dv", binary_path])
        .output()
        .expect("Failed to execute codesign");
    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get identifier for binary: {}", binary_path),
        )));
    }

    // Parse the output and extract the identifier
    let output_str = String::from_utf8(output.stderr).expect("Failed to convert output to string");
    let identifier = output_str
        .lines()
        .find(|line| line.starts_with("Identifier="))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim())
        .unwrap_or("Unknown");

    Ok(identifier.to_string())
}

// Get macho binary entitlements launching "codesign" command
pub fn get_macho_entitlements(binary_path: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
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

// Function that extracts external dependencies from a Mach-O binary
// launching "otool -L <binary_path>" command
pub fn get_macho_external_dependencies(
    binary_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

// Function that extracts binary imported symbols
// launching "nm -u <binary_path>" command
pub fn get_macho_imported_symbols(
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

pub trait FileType {
    fn is_macho(&self) -> bool;
}

impl FileType for PathBuf {
    fn is_macho(&self) -> bool {
        let file = match File::open(self) {
            Ok(file) => file,
            Err(_) => return false,
        };

        let mut buffer = [0; 4];
        if let Ok(_) = file.take(4).read_exact(&mut buffer) {
            // Check for Mach-O magic numbers
            buffer == [0xFE, 0xED, 0xFA, 0xCE] || // Mach-O 32-bit
            buffer == [0xCF, 0xFA, 0xED, 0xFE] || // Mach-O 32-bit (big-endian)
            buffer == [0xFE, 0xED, 0xFA, 0xCF] || // Mach-O 64-bit
            buffer == [0xCF, 0xFA, 0xED, 0xFE] || // Mach-O 64-bit (big-endian)
            buffer == [0xCA, 0xFE, 0xBA, 0xBE] // Mach-O universal binary
        } else {
            false
        }
    }
}
