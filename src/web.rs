use axum::{
    extract::Form,
    response::Html
};
use std::collections::HashMap;
use rusqlite::params;

use crate::utils::get_available_databases;


pub async fn index() -> Html<String> {
    let databases = get_available_databases();

    let db_options: String = databases
        .iter()
        .map(|db| format!(r#"<option value="{0}">{0}</option>"#, db))
        .collect();

    let html = format!(
        r#"
        <html>
            <head>
                <title>Dora - the explorer</title>
            </head>
            <body>
                <h1>Dora - the explorer</h1>
                <p>Explore macOS services</p>
                <form action="/query" method="post">
                    <label for="db">Choose database:</label>
                    <select name="db" id="db">
                        {db_options}
                    </select>
                    <br>
                    <label for="service">Service:</label>
                    <input type="text" name="service" id="service">
                    <br>
                    <label for="binary">Binary:</label>
                    <input type="text" name="binary" id="binary">
                    <br>
                    <label for="mach_service">Mach Service:</label>
                    <input type="text" name="mach_service" id="mach_service">
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
                    <button type="submit">Submit</button>
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
// • a service, it could be service label or mach service name
// • an entitlement name as "entitlement" key
// • a library name as "library" key
// • a symbol name as "symbol" key
// • a combination of the above.
pub async fn query(
    Form(input): Form<HashMap<String, String>>,
) -> Html<String> {
    // Extract the query parameters from the input
    let db = input.get("db").cloned().unwrap_or_default();



    let service = input.get("service").cloned().unwrap_or_default();
    let entitlement = input.get("entitlement").cloned().unwrap_or_default();
    let library = input.get("library").cloned().unwrap_or_default();
    let symbol = input.get("symbol").cloned().unwrap_or_default();

    // If service is populated and entitlement, library and symbols not,
    // only query by service param





    let conn = match rusqlite::Connection::open(&db) {
        Ok(conn) => conn,
        Err(e) => return Html(format!("Failed to open database: {}", e)),
    };

    // Query by service label ignoring case
    let sql_query = format!(
        "SELECT label, path, run_as_user, run_at_load, keep_alive, plist_path FROM service WHERE label LIKE ?1"
    );

    let mut stmt = match conn.prepare(&sql_query) {
        Ok(stmt) => stmt,
        Err(e) => return Html(format!("Failed to prepare SQL statement: {}", e)),
    };

    let result_set = match stmt.query_map(params![format!("%{}%", service)], |row| {
        Ok((
            row.get::<_, String>(0)?, // label
            row.get::<_, String>(1)?, // path
            row.get::<_, i32>(2)?,    // run_as_user
            row.get::<_, i32>(3)?,    // run_at_load
            row.get::<_, i32>(4)?,    // keep_alive
            row.get::<_, String>(5)?, // plist_path
        ))
    }) {
        Ok(rows) => rows,
        Err(e) => return Html(format!("Error executing query: {}", e)),
    };

    // Get entitlements of the service
    let ent_sql_query = format!(
        "SELECT e.name, se.value FROM entitlement e
         JOIN service_entitlement se ON e.id = se.entitlement_id
         WHERE se.service_id IN (SELECT id FROM service WHERE label LIKE ?1) ORDER BY e.name"
    );

    let mut ent_stmt = match conn.prepare(&ent_sql_query) {
        Ok(stmt) => stmt,
        Err(e) => {
            return Html(format!(
                "Failed to prepare entitlement SQL statement: {}",
                e
            ));
        }
    };

    let ent_result_set = match ent_stmt.query_map(params![format!("%{}%", service)], |row| {
        Ok((
            row.get::<_, String>(0)?, // entitlement name
            row.get::<_, String>(1)?, // entitlement value
        ))
    }) {
        Ok(rows) => rows,
        Err(e) => return Html(format!("Error executing entitlement query: {}", e)),
    };

    // Get libraries of the service
    let lib_sql_query = format!(
        "SELECT l.name, l.path FROM library l
         JOIN service_library sl ON l.id = sl.library_id
         WHERE sl.service_id IN (SELECT id FROM service WHERE label LIKE ?1) ORDER BY l.name"
    );

    let mut lib_stmt = match conn.prepare(&lib_sql_query) {
        Ok(stmt) => stmt,
        Err(e) => return Html(format!("Failed to prepare library SQL statement: {}", e)),
    };

    let lib_result_set = match lib_stmt.query_map(params![format!("%{}%", service)], |row| {
        Ok((
            row.get::<_, String>(0)?, // library name
            row.get::<_, String>(1)?, // library path
        ))
    }) {
        Ok(rows) => rows,
        Err(e) => return Html(format!("Error executing library query: {}", e)),
    };

    // Get symbols of the service
    let sym_sql_query = format!(
        "SELECT s.name FROM symbol s
         JOIN service_symbol ss ON s.id = ss.symbol_id
         WHERE ss.service_id IN (SELECT id FROM service WHERE label LIKE ?1) ORDER BY s.name"
    );
    let mut sym_stmt = match conn.prepare(&sym_sql_query) {
        Ok(stmt) => stmt,
        Err(e) => return Html(format!("Failed to prepare symbol SQL statement: {}", e)),
    };
    let sym_result_set = match sym_stmt.query_map(params![format!("%{}%", service)], |row| {
        Ok(row.get::<_, String>(0)?) // symbol name
    }) {
        Ok(rows) => rows,
        Err(e) => return Html(format!("Error executing symbol query: {}", e)),
    };

    // Here you can add logic to query the database based on the provided parameters
    // For now, we just return a success message with the query parameters
    Html(format!(
        r#"<html>
            <head>
                <title>Dora - the explorer</title>
            </head>
            <body>
                <h1>Dora - the explorer</h1>
                <p>Explore macOS services</p>
                <p>Database: {}</p>
                <p>Service: {}</p>
                <p>Entitlement: {}</p>
                <p>Library: {}</p>
                <p>Symbol: {}</p>
                <h2>Services:</h2>
                <ul>
                    {}
                </ul>
                <h2>Entitlements ():</h2>
                <ul>
                    {}
                </ul>
                <h2>Libraries:</h2>
                <ul>
                    {}
                </ul>
                <h2>Symbols:</h2>
                <ul>
                    {}
                </ul>
            </ul>
            </body>
        </html>"#,
        db, service, entitlement, library, symbol, result_set
            .map(|row| {
                match row {
                    Ok((label, path, run_as_user, run_at_load, keep_alive, plist_path)) => {
                        format!(
                            "<li>{} - {} (User: {}, RunAtLoad: {}, KeepAlive: {}, PlistPath: {})</li>",
                            label, path, run_as_user, run_at_load, keep_alive, plist_path
                        )
                    }
                    Err(e) => format!("<li>Error: {}</li>", e),
                }
            })
            .collect::<Vec<String>>()
            .join(""),
        ent_result_set
            .map(|row| {
                match row {
                    Ok((ent_name, ent_value)) => {
                        format!("<li>{}: {}</li>", ent_name, ent_value)
                    }
                    Err(e) => format!("<li>Error: {}</li>", e),
                }
            })
            .collect::<Vec<String>>()
            .join(""),
        lib_result_set
            .map(|row| {
                match row {
                    Ok((lib_name, lib_path)) => {
                        format!("<li>{} - {}</li>", lib_name, lib_path)
                    }
                    Err(e) => format!("<li>Error: {}</li>", e),
                }
            })
            .collect::<Vec<String>>()
            .join(""),
        sym_result_set
            .map(|row| {
                match row {
                    Ok(symbol_name) => {
                        format!("<li>{}</li>", symbol_name)
                    }
                    Err(e) => format!("<li>Error: {}</li>", e),
                }
            })
            .collect::<Vec<String>>()
            .join("")
    ))
}