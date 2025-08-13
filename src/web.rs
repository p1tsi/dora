use axum::{extract::Form, response::Html};
use std::collections::HashMap;

use crate::consts::{HTML_BODY_TITLE, HTML_FORM_FIELDS, HTML_HEADER};
use crate::sqlite::{
    get_entitlements_value_by_service_label, get_libraries_by_label, get_mach_service_by_label,
    get_service_by_label, get_services_by_entitlement, get_services_by_entitlement_and_symbol,
    get_services_by_label_pattern, get_services_by_library, get_services_by_symbol,
    get_symbols_by_label,
};
use crate::utils::{get_available_databases, is_valid_db};

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
    if is_valid_db(&db) == false {
        // If db is not valid, return an error message
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
        services_html = get_services_by_label_pattern(&db, &service).unwrap_or_else(|e| {
            eprintln!("Error retrieving service by label pattern: {}", e);
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

// For a given service label, get all entitlements, libraries, symbols and mach services associated with it.
pub async fn service(Form(input): Form<HashMap<String, String>>) -> Html<String> {
    let databases = get_available_databases();
    let db_options: String = databases
        .iter()
        .map(|db| format!(r#"<option value="{0}">{0}</option>"#, db))
        .collect();

    // Extract the query parameters from the input
    let db = input.get("db").cloned().unwrap_or_default();
    if is_valid_db(&db) == false {
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

    let service_label = input.get("label").cloned().unwrap_or_default();

    let conn = match rusqlite::Connection::open(&db) {
        Ok(conn) => conn,
        Err(e) => return Html(format!("Failed to open database: {}", e)),
    };

    let service_html = match get_service_by_label(&conn, &service_label) {
        Some((label, path, run_as_user, run_at_load, keep_alive, plist_path)) => {
            format!(
                "<ul>
                    <li><strong>Service:</strong> {}</li>
                    <li><strong>Path:</strong> {}</li>
                    <li><strong>Run as user:</strong> {}</li>
                    <li><strong>Run at load:</strong> {}</li>
                    <li><strong>Keep alive:</strong> {}</li>
                    <li><strong>Plist path:</strong> {}</li>
                </ul>",
                label, path, run_as_user, run_at_load, keep_alive, plist_path
            )
        }
        None => {
            format!(
                "<h2>Service: {}</h2>
                <p>No service found with label: {}</p>",
                service_label, service_label
            )
        }
    };

    // Get Mach services for the service
    let mach_services_html = match get_mach_service_by_label(&conn, &service_label) {
        Some(mach_services) => {
            if mach_services.is_empty() {
                "<h3>Mach Services:</h3><p>No Mach services found for this service.</p>".to_string()
            } else {
                let mach_services_count = mach_services.len();
                format!(
                    "<h3>Mach Services ({mach_services_count})</h3><ul>{}</ul>",
                    mach_services
                        .iter()
                        .map(|ms| format!("<li>{}</li>", ms))
                        .collect::<String>()
                )
            }
        }
        None => "<h3>Mach Services:</h3><p>Error retrieving Mach services.</p>".to_string(),
    };

    // Get entitlements for the service
    let entitlements_html = match get_entitlements_value_by_service_label(&conn, &service_label) {
        Some(entitlements) => {
            if entitlements.is_empty() {
                "<h3>Entitlements:</h3><p>No entitlements found for this service.</p>".to_string()
            } else {
                let entitlements_count = entitlements.len();
                format!(
                    "<h3>Entitlements ({entitlements_count})</h3><ul>{}</ul>",
                    entitlements
                        .iter()
                        .map(|(k, v)| format!("<li>{}: {}</li>", k, v))
                        .collect::<String>()
                )
            }
        }
        None => "<h3>Entitlements:</h3><p>Error retrieving entitlements.</p>".to_string(),
    };

    // Get libraries for the service
    let libraries_html = match get_libraries_by_label(&conn, &service_label) {
        Some(libraries) => {
            if libraries.is_empty() {
                "<h3>Libraries:</h3><p>No libraries found for this service.</p>".to_string()
            } else {
                let libraries_count = libraries.len();
                format!(
                    "<h3>Libraries ({libraries_count})</h3><ul>{}</ul>",
                    libraries
                        .iter()
                        .map(|(name, path)| format!("<li>{} ({})</li>", name, path))
                        .collect::<String>()
                )
            }
        }
        None => "<h3>Libraries:</h3><p>Error retrieving libraries.</p>".to_string(),
    };

    // Get symbols for the service
    let symbols_html = match get_symbols_by_label(&conn, &service_label) {
        Some(symbols) => {
            if symbols.is_empty() {
                "<h3>Symbols:</h3><p>No symbols found for this service.</p>".to_string()
            } else {
                let symbols_count = symbols.len();
                format!(
                    "<h3>Symbols ({symbols_count})</h3><ul>{}</ul>",
                    symbols
                        .iter()
                        .map(|s| format!("<li>{}</li>", s))
                        .collect::<String>()
                )
            }
        }
        None => "<h3>Symbols:</h3><p>Error retrieving symbols.</p>".to_string(),
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
