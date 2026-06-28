-- 0002: 项目与素材勾选表。
--
-- 注意：原 selections 表含 package_id 列（两级视图整包勾选），随 0006 迁移改为 directory_id。
-- 这里更新为新结构（scope IN directory/file/exclude），与 0006 重建后的 schema 对齐，
-- 避免每次启动时 CREATE INDEX 引用已不存在的 package_id 列导致报错闪退。

CREATE TABLE IF NOT EXISTS projects (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    export_root TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS selections (
    id           INTEGER PRIMARY KEY,
    scope        TEXT NOT NULL CHECK(scope IN ('directory','file','exclude')),
    directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE,
    file_id      INTEGER REFERENCES files(id) ON DELETE CASCADE,
    project_id   INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at   INTEGER NOT NULL,
    CHECK((scope='directory' AND directory_id IS NOT NULL) OR
          (scope IN ('file','exclude') AND file_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_selections_project ON selections(project_id);
CREATE INDEX IF NOT EXISTS idx_selections_directory ON selections(directory_id);
CREATE INDEX IF NOT EXISTS idx_selections_file ON selections(file_id);
