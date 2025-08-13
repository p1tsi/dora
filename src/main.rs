use axum::{
    Router,
    routing::{get, post},
};

mod consts;
mod macho;
mod sqlite;
mod utils;
mod web;

use crate::sqlite::populate_db;
use crate::utils::generate_sqlite_filename;
use consts::{LISTENING_ADDRESS, LISTENING_PORT};
use web::*;

// Print banner for "dora" tool
fn print_banner() {
    println!(
        r#"
  _____                  
 |  __ \                 
 | |  | | ___  _ __ __ _ 
 | |  | |/ _ \| '__/ _` |
 | |__| | (_) | | | (_| |
 |_____/ \___/|_|  \__,_|         
                     
"#
    );
    println!(
        "\tA macOS attack surface explorer - v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("\tAuthor: {}", env!("CARGO_PKG_AUTHORS"));
    println!("\tGitHub: {}", env!("CARGO_PKG_REPOSITORY"));
    println!();
}

// Main function that orchestrates the database creation, plist parsing, and data extraction
#[tokio::main]
async fn main() {
    print_banner();

    let database_path = generate_sqlite_filename();
    if !std::path::Path::new(&database_path).exists() {
        // Create the SQLite database file
        println!("Creating SQLite database file: {}", database_path);

        populate_db(&database_path)
            .expect("Failed to populate the database with services and their data");
    }

    // Start the web server to serve the data
    println!(
        "Dora is running at http://{}:{}",
        LISTENING_ADDRESS, LISTENING_PORT
    );

    let app = Router::new()
        .route("/", get(index))
        .route("/query", post(query))
        .route("/service", get(service));

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", LISTENING_ADDRESS, LISTENING_PORT))
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}
