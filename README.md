# dora – macOS Attack Surface Explorer

Ever wondered which macOS system services have specific **entitlements**?  
Or which services are **code-signed** with a particular entitlement?  
Or maybe which services **import a certain library or symbol**?

That's exactly what **dora** helps you answer — quickly and efficiently.

## What is dora?

**dora** is a Rust-based tool that collects and organizes metadata about macOS system services, allowing you to explore the system's attack surface with ease.

On first launch, `dora` scans all property list files in:

- `/System/Library/LaunchDaemons`
- `/System/Library/LaunchAgents`

It extracts key information and builds a searchable **SQLite database**.

Once the database is built, simply open your browser and go to http://127.0.0.1:8778

Here, you'll find a web interface where you can:

- Search services by entitlement
- Find services signed with a specific entitlement
- Discover what libraries or symbols services import


## Getting Started

Clone the repo and build the project:

```bash
git clone https://github.com/p1tsi/dora.git
cd dora
cargo build --release
```

Then run it:

```bash
./target/release/dora
```

