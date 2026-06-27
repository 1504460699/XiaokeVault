CREATE TABLE IF NOT EXISTS duplicate_groups (
    id          INTEGER PRIMARY KEY,
    reason      TEXT NOT NULL,       -- zip_extracted / likely_backup / hash
    detail      TEXT,                -- 人类可读说明
    hash        TEXT,                -- reason=hash 时填
    created_at  INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS duplicate_members (
    id          INTEGER PRIMARY KEY,
    group_id    INTEGER NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
    file_id     INTEGER REFERENCES files(id) ON DELETE CASCADE,
    package_id  INTEGER REFERENCES packages(id) ON DELETE CASCADE,
    role        TEXT NOT NULL DEFAULT 'remove',  -- keep / remove
    UNIQUE(group_id, file_id, package_id)
);
CREATE INDEX IF NOT EXISTS idx_dup_members_group ON duplicate_members(group_id);
