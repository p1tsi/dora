PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Service table --
-- This table is used to store information about services.
CREATE TABLE IF NOT EXISTS service (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    run_as_user TEXT,
    run_at_load TEXT,
    keep_alive TEXT, 
    plist_path TEXT UNIQUE
);

-- Mach service table --
-- This table is used to store Mach services associated with a service.
CREATE TABLE IF NOT EXISTS mach_service (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE,
    value INTEGER,
    service_id INTEGER,
    FOREIGN KEY (service_id) REFERENCES service(id)
);

-- Entitlement table --
-- This table is used to store entitlements associated with services.        
CREATE TABLE IF NOT EXISTS entitlement (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);


-- Service entitlement table --
-- This table is used to associate services with their entitlements.
-- It creates a many-to-many relationship between services and entitlements.
-- Each service can have multiple entitlements, and each entitlement can be associated with multiple services.
CREATE TABLE IF NOT EXISTS service_entitlement (
    service_id INTEGER,
    entitlement_id INTEGER,
    value TEXT,
    PRIMARY KEY (service_id, entitlement_id, value),
    FOREIGN KEY (service_id) REFERENCES service(id),
    FOREIGN KEY (entitlement_id) REFERENCES entitlement(id)
);

-- Library table --
CREATE TABLE IF NOT EXISTS library (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL UNIQUE
);

-- Library service table --
-- This table is used to associate libraries with services.
-- It creates a many-to-many relationship between libraries and services.
-- Each library can be associated with multiple services, and each service can have multiple libraries.
CREATE TABLE IF NOT EXISTS service_library (
    service_id INTEGER,
    library_id INTEGER,
    PRIMARY KEY (service_id, library_id),
    FOREIGN KEY (service_id) REFERENCES service(id),
    FOREIGN KEY (library_id) REFERENCES library(id)
);


-- Symbol table --
CREATE TABLE IF NOT EXISTS symbol (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

-- Symbol service table --
-- This table is used to associate symbols with services.
-- It creates a many-to-many relationship between symbols and services.
-- Each symbol can be associated with multiple services, and each service can have multiple symbols.
CREATE TABLE IF NOT EXISTS service_symbol (
    service_id INTEGER,
    symbol_id INTEGER,
    PRIMARY KEY (service_id, symbol_id),
    FOREIGN KEY (service_id) REFERENCES service(id),
    FOREIGN KEY (symbol_id) REFERENCES symbol(id)
);