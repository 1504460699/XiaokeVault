-- 目录树表（自关联树结构，与 categories/packages 并存）
CREATE TABLE IF NOT EXISTS directories (
    id          INTEGER PRIMARY KEY,
    library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    parent_id   INTEGER REFERENCES directories(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL DEFAULT 0,
    file_count  INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    UNIQUE(library_id, path)
);
CREATE INDEX IF NOT EXISTS idx_dirs_parent ON directories(parent_id);
CREATE INDEX IF NOT EXISTS idx_dirs_library ON directories(library_id);
-- files 表增加 directory_id 的 ALTER 在 Rust migrate() 里幂等执行（SQLite 不支持 ADD COLUMN IF NOT EXISTS）

