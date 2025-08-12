use axum::{extract::Form, response::Html};
use rusqlite::params;
use std::collections::HashMap;

use crate::consts::*;
use crate::utils::get_available_databases;

// Get services from SQLite database that have a sepcified entitlement AND
// a specified symbol
fn get_services_by_entitlement_and_symbol(
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

pub async fn index() -> Html<String> {
    let databases = get_available_databases();

    let db_options: String = databases
        .iter()
        .map(|db| format!(r#"<option value="{0}">{0}</option>"#, db))
        .collect();

    let html = format!(
        r#"
        <html>
            {HTML_HEADER}
            <body>
                {HTML_BODY_TITLE}
                <form action="/query" method="post">
                    <label for="db">Choose database:</label>
                    <select name="db" id="db">
                        {db_options}
                    </select>
                    {HTML_FORM_FIELDS}
                </form>
            </body>
        </html>
        "#
    );

    Html(html)
}

// Get service from SQLite database by label case insensitive.
// Handle multiple services retrieved by label.
fn get_service_by_label_pattern(
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

// Get all services from SQLite database having a specific entitlement.
// Handle multiple services retrieved by entitlement.
// The HTML output should contain also the count of services found.
fn get_services_by_entitlement(
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

// Get all services from SQLite database importing a specific library.
// Handle multiple services retrieved by library.
fn get_services_by_library(db: &String, library: &str) -> Result<Vec<String>, rusqlite::Error> {
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

// Get all services from SQLite database having a specific symbol.
// Handle multiple services retrieved by symbol.
fn get_services_by_symbol(db: &String, symbol: &str) -> Result<Vec<String>, rusqlite::Error> {
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

// Handler for the "/query" route
// This route is used to query the database with a SQL query provided by the user
// The user could submit:
// • a service label as "service" key
// • an entitlement name as "entitlement" key
// • a library name as "library" key
// • a symbol name as "symbol" key
// • a combination of the above.
pub async fn query(Form(input): Form<HashMap<String, String>>) -> Html<String> {
    let databases = get_available_databases();

    let db_options: String = databases
        .iter()
        .map(|db| format!(r#"<option value="{0}">{0}</option>"#, db))
        .collect();

    // Extract the query parameters from the input
    let db = input.get("db").cloned().unwrap_or_default();

    // Make sure db is not empty, starts with "dora_", ends with ".sqlite" and not contains "/" character.
    if db.is_empty() || !db.starts_with("dora_") || !db.ends_with(".sqlite") || db.contains('/') {
        return Html(format!(
            r#"<html>
                {HTML_HEADER}
                <body>
                    {HTML_BODY_TITLE}
                    <p>Invalid database name: {}</p>
                    <form action="/query" method="post">
                        <label for="db">Choose database:</label>
                        <select name="db" id="db">
                            {db_options}
                        </select>
                        {HTML_FORM_FIELDS}
                    </form>
                </body>
            </html>"#,
            db
        ));
    }

    let service = input.get("service").cloned().unwrap_or_default();
    let entitlement = input.get("entitlement").cloned().unwrap_or_default();
    let library = input.get("library").cloned().unwrap_or_default();
    let symbol = input.get("symbol").cloned().unwrap_or_default();

    let mut services_html: String = "<p>No query parameters provided.</p>".to_string();

    if !service.is_empty() {
        services_html = get_service_by_label_pattern(&db, &service).unwrap_or_else(|e| {
            eprintln!("Error retrieving service by label: {}", e);
            "<p>Error retrieving service.</p>".to_string()
        });
    } else if !entitlement.is_empty() {
        if !symbol.is_empty() {
            // If both entitlement and symbol are provided, get services by both
            let services = get_services_by_entitlement_and_symbol(&db, &entitlement, &symbol)
                .unwrap_or_else(|e| {
                    eprintln!("Error retrieving services by entitlement and symbol: {}", e);
                    vec![format!("<p>Error retrieving services.</p>")]
                });

            services_html = services.join("\n");
        } else {
            let services = get_services_by_entitlement(&db, &entitlement).unwrap_or_else(|e| {
                eprintln!("Error retrieving services by entitlement: {}", e);
                vec![format!("<p>Error retrieving services.</p>")]
            });

            services_html = services.join("\n");
        }
    } else if !library.is_empty() {
        let services = get_services_by_library(&db, &library).unwrap_or_else(|e| {
            eprintln!("Error retrieving services by library: {}", e);
            vec![format!("<p>Error retrieving services.</p>")]
        });

        services_html = services.join("\n");
    } else if !symbol.is_empty() {
        let services = get_services_by_symbol(&db, &symbol).unwrap_or_else(|e| {
            eprintln!("Error retrieving services by symbol: {}", e);
            vec![format!("<p>Error retrieving services.</p>")]
        });

        services_html = services.join("\n");
    }

    return Html(format!(
        r#"<html>
            {HTML_HEADER}
            <body>
                {HTML_BODY_TITLE}
                <form action="/query" method="post">
                    <label for="db">Choose database:</label>
                    <select name="db" id="db">
                        {db_options}
                    </select>
                    {HTML_FORM_FIELDS}
                </form>
                <h2>Using: {db}</h2>
                <ul>{services_html}</ul>
            </body>
        </html>"#
    ));
}

// Get all service column from SQLite database by label.
fn get_service_by_label(
    conn: &rusqlite::Connection,
    label: &str,
) -> Result<String, rusqlite::Error> {
    let mut stmt = conn.prepare(SERVICE_BY_LABEL)?;
    let result_set = stmt.query_map(params![label], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
            row.get::<_, i32>(2)?,    // run_as_user
            row.get::<_, i32>(3)?,    // run_at_load
            row.get::<_, i32>(4)?,    // keep_alive
            row.get::<_, String>(5)?, // plist_path
        ))
    })?;
    let mut services = Vec::new();
    for service in result_set {
        match service {
            Ok((label, path, run_as_user, run_at_load, keep_alive, plist_path)) => {
                services.push(format!(
                    "<ul>
                        <li><strong>Service:</strong> {label}</li>
                        <li><strong>Path:</strong> {path}</li>
                        <li><strong>Run as user:</strong> {run_as_user}</li>
                        <li><strong>Run at load:</strong> {run_at_load}</li>
                        <li><strong>Keep alive:</strong> {keep_alive}</li>
                        <li><strong>Plist path:</strong> {plist_path}</li>
                    </ul>"
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving service by label: {}", e);
                return Err(e);
            }
        }
    }
    if services.is_empty() {
        return Ok(format!("<p>No service found with label: {}</p>", label));
    }
    let mut html = String::new();
    for service in services {
        html.push_str(&service);
    }

    Ok(html)
}

// For a given service label, get all entitlements, libraries, symbols and mach services associated with it.
pub async fn service(Form(input): Form<HashMap<String, String>>) -> Html<String> {
    let databases = get_available_databases();

    let db_options: String = databases
        .iter()
        .map(|db| format!(r#"<option value="{0}">{0}</option>"#, db))
        .collect();

    // Extract the query parameters from the input
    let db = input.get("db").cloned().unwrap_or_default();
    let service_label = input.get("label").cloned().unwrap_or_default();

    let conn = match rusqlite::Connection::open(&db) {
        Ok(conn) => conn,
        Err(e) => return Html(format!("Failed to open database: {}", e)),
    };

    // Get service details by label
    let service_html = get_service_by_label(&conn, &service_label).unwrap_or_else(|e| {
        eprintln!("Error retrieving service by label: {}", e);
        "<p>Error retrieving service.</p>".to_string()
    });

    // Get mach services for the service
    let mut stmt = conn.prepare(MACH_SERVICES_BY_LABEL).unwrap();
    let mach_services: Vec<String> = stmt
        .query_map(params![service_label], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    let mach_services_html = if mach_services.is_empty() {
        "<h3>Mach Services:</h3><p>No Mach services found for this service.</p>".to_string()
    } else {
        format!(
            "<h3>Mach Services:</h3><ul>{}</ul>",
            mach_services
                .iter()
                .map(|ms| format!("<li>{}</li>", ms))
                .collect::<String>()
        )
    };

    // Get entitlements for the service
    let mut stmt = conn.prepare(ENTITLEMENTS_VALUE_BY_SERVICE_LABEL).unwrap();
    let entitlements: Vec<(String, String)> = stmt
        .query_map(params![service_label], |row| {
            Ok((row.get(0)?, row.get(1)?)) // (entitlement_name, entitlement_value)
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    let entitlements_html = if entitlements.is_empty() {
        "<h3>Entitlements:</h3><p>No entitlements found for this service.</p>".to_string()
    } else {
        format!(
            "<h3>Entitlements:</h3><ul>{}</ul>",
            entitlements
                .iter()
                .map(|(name, value)| format!("<li>{name}: {value}</li>"))
                .collect::<String>()
        )
    };

    // Get libraries for the service
    let mut stmt = conn.prepare(LIBRARIES_BY_LABEL).unwrap();
    let libraries: Vec<(String, String)> = stmt
        .query_map(params![service_label], |row| {
            Ok((row.get(0)?, row.get(1)?)) // (name, path)
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let libraries_html = if libraries.is_empty() {
        "<p>No libraries found for this service.</p>".to_string()
    } else {
        format!(
            "<h3>Libraries:</h3><ul>{}</ul>",
            libraries
                .iter()
                .map(|(name, path)| format!("<li>{name} ({path})</li>"))
                .collect::<String>()
        )
    };

    // Get symbols for the service
    let mut stmt = conn.prepare(SYMBOLS_BY_LABEL).unwrap();
    let symbols: Vec<String> = stmt
        .query_map(params![service_label], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let symbols_html = if symbols.is_empty() {
        "<p>No symbols found for this service.</p>".to_string()
    } else {
        format!(
            "<h3>Symbols:</h3><ul>{}</ul>",
            symbols
                .iter()
                .map(|s| format!("<li>{}</li>", s))
                .collect::<String>()
        )
    };

    // Combine all HTML parts
    let html = format!(
        r#"<html>
            {HTML_HEADER}
            <body>
                {HTML_BODY_TITLE}
                <form action="/query" method="post">
                    <label for="db">Choose database:</label>
                    <select name="db" id="db">
                        {db_options}
                    </select>
                    {HTML_FORM_FIELDS}
                </form>
                <h2>Using: {db}</h2>
                <p>{service_html}</p>
                <p>{mach_services_html}</p>
                <p>{entitlements_html}</p>
                <p>{libraries_html}</p>
                <p>{symbols_html}</p>
            </body>
        </html>"#
    );

    Html(html)
}
