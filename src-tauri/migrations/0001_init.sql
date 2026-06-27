CREATE TABLE IF NOT EXISTS libraries (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    root_path    TEXT NOT NULL UNIQUE,
    created_at   INTEGER NOT NULL,
    last_scan_at INTEGER
);

CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY,
    library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    sort_order  INTEGER NOT NULL,
    UNIQUE(library_id, name)
);

CREATE TABLE IF NOT EXISTS packages (
    id             INTEGER PRIMARY KEY,
    category_id    INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    path           TEXT NOT NULL,
    file_count     INTEGER DEFAULT 0,
    total_bytes    INTEGER DEFAULT 0,
    has_zip        INTEGER DEFAULT 0,
    source_url     TEXT,
    source_title   TEXT,
    license        TEXT,
    license_source TEXT,
    UNIQUE(category_id, name)
);

CREATE TABLE IF NOT EXISTS files (
    id           INTEGER PRIMARY KEY,
    package_id   INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    rel_path     TEXT NOT NULL,
    name         TEXT NOT NULL,
    ext          TEXT NOT NULL,
    kind         TEXT NOT NULL,
    bytes        INTEGER NOT NULL,
    width        INTEGER,
    height       INTEGER,
    frame_count  INTEGER,
    modified_at  INTEGER NOT NULL,
    content_hash TEXT,
    deleted      INTEGER DEFAULT 0,
    UNIQUE(package_id, rel_path)
);

CREATE TABLE IF NOT EXISTS asset_types (
    kind        TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    extensions  TEXT NOT NULL,
    viewer      TEXT NOT NULL,
    icon        TEXT,
    is_source   INTEGER DEFAULT 0,
    built_in    INTEGER DEFAULT 0,
    sort_order  INTEGER DEFAULT 0
);
