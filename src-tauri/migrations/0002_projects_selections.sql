CREATE TABLE IF NOT EXISTS projects (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    export_root TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS selections (
    id          INTEGER PRIMARY KEY,
    scope       TEXT NOT NULL CHECK(scope IN ('package','file','exclude')),
    package_id  INTEGER REFERENCES packages(id) ON DELETE CASCADE,
    file_id     INTEGER REFERENCES files(id) ON DELETE CASCADE,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL,
    CHECK((scope='package' AND package_id IS NOT NULL) OR
          (scope IN ('file','exclude') AND file_id IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS idx_selections_project ON selections(project_id);
CREATE INDEX IF NOT EXISTS idx_selections_package ON selections(package_id);
CREATE INDEX IF NOT EXISTS idx_selections_file ON selections(file_id);
