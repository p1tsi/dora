use axum::{
    Router,
    routing::{get, post},
};

mod consts;
mod database;
mod sqlite;
mod utils;
mod web;

use consts::{LISTENING_ADDRESS, LISTENING_PORT};
use database::populate_db;
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

    // Check if the SQLite database file already exists
    if !std::path::Path::new(&sqlite_filename).exists() {
        // Create the SQLite database file
        println!("Creating SQLite database file: {}", sqlite_filename);

        populate_db(&sqlite_filename)
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
