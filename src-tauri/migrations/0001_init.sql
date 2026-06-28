-- 初始建表迁移。
-- 注意：原 categories / packages 两级视图表已随 0006 迁移移除，这里不再创建。
-- 历史库（含旧表）由 0006 迁移负责清理。
-- 新建库直接用目录树架构（directories + files.directory_id）。

CREATE TABLE IF NOT EXISTS libraries (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    root_path   TEXT NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL,
    last_scan_at INTEGER
);

-- files 表：目录树架构，文件经 directory_id 关联目录。
-- 注意：早期版本此表含 package_id 列（两级视图），0005/0006 迁移已重建为去掉该列。
-- 对全新库，这里直接建新结构；对历史库，IF NOT EXISTS 会跳过，由 0005/0006 负责重建。
CREATE TABLE IF NOT EXISTS files (
    id           INTEGER PRIMARY KEY,
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
    directory_id INTEGER NOT NULL REFERENCES directories(id) ON DELETE CASCADE,
    UNIQUE(directory_id, rel_path)
);

-- 资源类型表：扩展名 → kind 分类映射，可由用户自定义。
CREATE TABLE IF NOT EXISTS asset_types (
    kind       TEXT PRIMARY KEY,
    label      TEXT NOT NULL,
    extensions TEXT NOT NULL,        -- JSON 数组字符串
    viewer     TEXT NOT NULL,        -- image/gif/audio/text/model/font/source
    icon       TEXT,
    is_source  INTEGER DEFAULT 0
);
