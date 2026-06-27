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

-- files 表增加 directory_id（关联直接所在目录）
-- 注意：SQLite 的 ALTER TABLE ADD COLUMN 不能加到已有表的非末尾，此处加在末尾即可
ALTER TABLE files ADD COLUMN directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_files_directory ON files(directory_id);
