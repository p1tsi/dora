// HTML Constants
pub const HTML_HEADER: &str = "
    <head>
        <title>Dora - the explorer</title>
    </head>";

pub const HTML_BODY_TITLE: &str = "
    <h1>Dora - the explorer</h1>
    <p>Explore macOS services</p>";

pub const HTML_FORM_FIELDS: &str = r#"<br>
                    <label for="service">Service:</label>
                    <input type="text" name="service" id="service">
                    <br>
                    <label for="entitlement">Entitlement:</label>
                    <input type="text" name="entitlement" id="entitlement">
                    <br>
                    <label for="library">Library:</label>
                    <input type="text" name="library" id="library">
                    <br>
                    <label for="symbol">Symbol:</label>
                    <input type="text" name="symbol" id="symbol">
                    <br>
                    <button type="submit">Submit</button>"#;

// Web server IP and port
pub static LISTENING_ADDRESS: &str = "127.0.0.1";
pub static LISTENING_PORT: u16 = 8778;

// SQLite queries

// Insert queries
pub const INSERT_MACH_SERVICE: &str =
    "INSERT OR IGNORE INTO mach_service (name, value, service_id) VALUES (?1, ?2, ?3)";

pub const INSERT_SERVICE_ENTITLEMENT: &str = "INSERT OR IGNORE INTO service_entitlement (service_id, entitlement_id, value) VALUES (?1, ?2, ?3)";

pub const INSERT_LIBRARY: &str =
    "INSERT OR IGNORE INTO service_library (service_id, library_id) VALUES (?1, ?2)";

pub const INSERT_SYMBOL: &str =
    "INSERT OR IGNORE INTO service_symbol (service_id, symbol_id) VALUES (?1, ?2)";

// Select queries
pub const SERVICES_BY_ENTITLEMENT_AND_SYMBOL: &str = "SELECT DISTINCT s.label, s.path \
     FROM service s \
     JOIN service_entitlement se ON s.id = se.service_id \
     JOIN entitlement e ON se.entitlement_id = e.id \
     JOIN service_symbol ss ON s.id = ss.service_id \
     JOIN symbol sy ON ss.symbol_id = sy.id \
     WHERE e.name LIKE ?1 COLLATE NOCASE AND sy.name GLOB ?2 ORDER BY s.label";

pub const SERVICES_BY_LABEL_PATTERN: &str = "SELECT DISTINCT s.label, s.path \
     FROM service s \
     WHERE s.label GLOB ?1 ORDER BY s.label";

pub const SERVICE_BY_LABEL: &str = "SELECT s.label, s.path, s.run_as_user, s.run_at_load, s.keep_alive, s.plist_path \
     FROM service s \
     WHERE s.label = ?1 COLLATE NOCASE";

pub const SERVICES_BY_ENTITLEMENT: &str = "SELECT DISTINCT s.label, s.path \
     FROM service s \
     JOIN service_entitlement se ON s.id = se.service_id \
     JOIN entitlement e ON se.entitlement_id = e.id \
     WHERE e.name LIKE ?1 COLLATE NOCASE ORDER BY s.label";

pub const SERVICES_BY_LIBRARY: &str = "SELECT DISTINCT s.label, s.path \
     FROM service s \
     JOIN service_library sl ON s.id = sl.service_id \
     JOIN library l ON sl.library_id = l.id \
     WHERE l.name LIKE ?1 COLLATE NOCASE ORDER BY s.label";

pub const SERVICES_BY_SYMBOL: &str = "SELECT DISTINCT s.label, s.path \
     FROM service s \
     JOIN service_symbol ss ON s.id = ss.service_id \
     JOIN symbol sy ON ss.symbol_id = sy.id \
     WHERE sy.name GLOB ?1 ORDER BY s.label";

pub const MACH_SERVICES_BY_LABEL: &str = "SELECT ms.name FROM mach_service ms \
     JOIN service s ON s.id = ms.service_id \
     WHERE s.label = ?1 COLLATE NOCASE";

pub const ENTITLEMENTS_VALUE_BY_SERVICE_LABEL: &str = "SELECT e.name AS entitlement_name, se.value AS entitlement_value \
     FROM service s \
     JOIN service_entitlement se ON s.id = se.service_id \
     JOIN entitlement e ON se.entitlement_id = e.id \
     WHERE s.label = ?1 COLLATE NOCASE";

pub const LIBRARIES_BY_LABEL: &str = "SELECT l.name, l.path FROM library l \
     JOIN service_library sl ON l.id = sl.library_id \
     JOIN service s ON sl.service_id = s.id \
     WHERE s.label = ?1 COLLATE NOCASE ORDER BY l.name";

pub const SYMBOLS_BY_LABEL: &str = "SELECT sy.name FROM symbol sy \
     JOIN service_symbol ss ON sy.id = ss.symbol_id \
     JOIN service s ON ss.service_id = s.id \
     WHERE s.label = ?1 COLLATE NOCASE ORDER BY sy.name";
