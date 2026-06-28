use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Connection, SqlitePool};
use std::path::PathBuf;
use std::str::FromStr;

/// 应用数据目录名（变更：tauri-app → vault）。
/// 旧目录 com.xiaoke.tauri-app 会在 connect() 时自动迁移到新目录。
pub const DATA_DIR: &str = "com.xiaoke.vault";
const LEGACY_DATA_DIR: &str = "com.xiaoke.tauri-app";

/// 返回数据库文件路径：<app_data>/com.xiaoke.vault/index.db
pub fn db_path() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push(DATA_DIR);
    std::fs::create_dir_all(&p).expect("create app data dir");
    p.push("index.db");
    p
}

/// 应用数据根目录（供缩略图/回收站等共用）。
pub fn data_root() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push(DATA_DIR);
    std::fs::create_dir_all(&p).expect("create app data dir");
    p
}

/// 一次性迁移：若新目录为空而旧目录存在，把旧目录整体搬到新目录。
/// 保留已扫描的 index.db、缩略图缓存、去重复份等。
pub fn migrate_legacy_data() {
    let new_root = data_root();
    let mut old = dirs::data_dir().expect("no data dir");
    old.push(LEGACY_DATA_DIR);
    if !old.exists() {
        return;
    }
    // 新目录若已有 index.db，说明已是新数据，不覆盖
    if new_root.join("index.db").exists() {
        return;
    }
    log::info!("[db] 迁移旧数据目录 {} → {}", old.display(), new_root.display());
    // 新目录此时一般只有刚 create_dir_all 建的空壳，先删掉再整体 rename
    let _ = std::fs::remove_dir_all(&new_root);
    if let Err(e) = std::fs::rename(&old, &new_root) {
        // rename 失败（跨卷等），回退到递归拷贝
        log::warn!("[db] rename 失败({e})，尝试递归拷贝");
        if let Err(e) = copy_dir_recursive(&old, &new_root) {
            log::error!("[db] 迁移失败：{e}");
        }
    }
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// 创建连接池（启用外键）。
/// 注意：必须先迁移旧目录数据，再创建新库，否则空库会被误判为「已存在」而跳过迁移。
pub async fn connect() -> Result<SqlitePool, sqlx::Error> {
    migrate_legacy_data();
    let path = db_path();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
}

/// 执行初始建表迁移。
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(include_str!("../migrations/0001_init.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("../migrations/0002_projects_selections.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("../migrations/0003_dedup.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("../migrations/0004_directories.sql"))
        .execute(pool)
        .await?;
    // files.directory_id：SQLite 不支持 ADD COLUMN IF NOT EXISTS，先查列是否存在再加
    ensure_column(pool, "files", "directory_id", "INTEGER REFERENCES directories(id) ON DELETE CASCADE").await?;
    // 0005：修复 files 表唯一约束（package_id,rel_path)→(directory_id,rel_path)
    // 解决树扫描同名文件互相覆盖导致目录显示空的问题
    fix_files_unique_constraint(pool).await?;
    // 0006：移除两级视图，统一目录树架构
    // drop categories/packages/dedup 表，directories 加版权列，selections/files 重建，清理旧两级数据
    drop_two_level_schema(pool).await?;
    Ok(())
}

/// 0005 迁移：修复 files 表唯一约束。
///
/// 旧约束 UNIQUE(package_id, rel_path)：树扫描所有文件 package_id=0、
/// rel_path 相对目录，导致不同目录同名文件互相覆盖。
/// 新约束 UNIQUE(directory_id, rel_path)：每目录内唯一；两级文件 directory_id=NULL
/// 不冲突（SQLite NULL 语义），两级扫描改用显式去重。
///
/// 幂等：若约束已是 (directory_id, rel_path) 则跳过。
/// 风险控制：迁移前复制 index.db → index.db.bak。
async fn fix_files_unique_constraint(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 用 schema_migrations 标记表实现幂等，避免依赖脆弱的 LIKE 匹配
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        )",
    )
    .execute(pool)
    .await?;
    let applied: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM schema_migrations WHERE version=5",
    )
    .fetch_one(pool)
    .await?;
    if applied > 0 {
        log::debug!("[db] 0005 迁移：已应用，跳过");
        return Ok(());
    }

    log::info!("[db] 0005 迁移：重建 files 表，修正唯一约束");
    let path = db_path();
    let backup = path.with_extension("db.bak");
    if let Err(e) = std::fs::copy(&path, &backup) {
        log::warn!("[db] 0005 备份失败（继续迁移）：{e}");
    } else {
        log::info!("[db] 0005 已备份数据库 → {}", backup.display());
    }

    let mut conn = pool.acquire().await?;

    // 整个重建在一个事务里，失败则回滚（备份文件兜底）
    let mut tx = conn.begin().await?;

    // 清理：删除所有 package_id=0 的旧脏树文件（被覆盖的同名文件）
    let dirty: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE package_id=0")
            .fetch_one(&mut *tx)
            .await?;
    log::info!("[db] 0005 清理旧脏树文件（package_id=0）共 {} 条", dirty);
    sqlx::query("DELETE FROM files WHERE package_id=0")
        .execute(&mut *tx)
        .await?;

    // 重建表：新约束 UNIQUE(directory_id, rel_path)
    sqlx::query(
        "CREATE TABLE files_new (
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
            directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE,
            UNIQUE(directory_id, rel_path)
        )",
    )
    .execute(&mut *tx)
    .await?;

    // 迁移数据：旧两级文件（package_id≠0, directory_id=NULL）原样保留
    sqlx::query(
        "INSERT INTO files_new
            (id, package_id, rel_path, name, ext, kind, bytes, width, height,
             frame_count, modified_at, content_hash, deleted, directory_id)
         SELECT id, package_id, rel_path, name, ext, kind, bytes, width, height,
                frame_count, modified_at, content_hash, deleted, directory_id
         FROM files",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("DROP TABLE files").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE files_new RENAME TO files").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_directory_id ON files(directory_id)")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    // 记录迁移已应用（幂等标记）
    sqlx::query("INSERT OR IGNORE INTO schema_migrations(version) VALUES (5)")
        .execute(pool)
        .await?;
    log::info!("[db] 0005 迁移完成：files 表已重建，约束 UNIQUE(directory_id, rel_path)");
    log::info!("[db] 0005 提示：应用启动后需重新扫描树以填充 directory_id");
    Ok(())
}

/// 幂等添加列：若列不存在则 ALTER TABLE ADD COLUMN，并建同名索引。
async fn ensure_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    def: &str,
) -> Result<(), sqlx::Error> {
    let exists: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name='{}'",
        table, column
    ))
    .fetch_one(pool)
    .await?;
    if exists.0 == 0 {
        sqlx::query(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, def))
            .execute(pool)
            .await?;
        sqlx::query(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({})",
            table, column, table, column
        ))
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// 0006 迁移：移除两级视图（categories/packages），统一为目录树架构。
///
/// 改动：
/// - directories 表加版权列（source_url/source_title/license/license_source）
/// - files 表重建：去掉 package_id 列与外键，directory_id 改 NOT NULL，UNIQUE(directory_id, rel_path)
/// - 清理旧两级数据：DELETE FROM files WHERE directory_id IS NULL
/// - selections 表重建：package_id → directory_id，scope 'package' → 'directory'
/// - DROP: categories / packages / duplicate_groups / duplicate_members / dismissed_pairs
///
/// 幂等：schema_migrations version=6。安全：迁移前 index.db → index.db.bak。
async fn drop_two_level_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 幂等检查
    let applied: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM schema_migrations WHERE version=6",
    )
    .fetch_one(pool)
    .await?;
    if applied > 0 {
        log::debug!("[db] 0006 迁移：已应用，跳过");
        return Ok(());
    }

    log::info!("[db] 0006 迁移：移除两级视图，统一目录树架构");
    let path = db_path();
    let backup = path.with_extension("db.bak2");
    if let Err(e) = std::fs::copy(&path, &backup) {
        log::warn!("[db] 0006 备份失败（继续迁移）：{e}");
    } else {
        log::info!("[db] 0006 已备份数据库 → {}", backup.display());
    }

    let mut conn = pool.acquire().await?;
    // 整个迁移在一个事务里，失败则回滚（备份文件兜底）
    let mut tx = conn.begin().await?;

    // 1) directories 加版权列（幂等 ADD COLUMN，SQLite 无 IF NOT EXISTS，用 pragma 检查）
    for (col, def) in [
        ("source_url", "TEXT"),
        ("source_title", "TEXT"),
        ("license", "TEXT"),
        ("license_source", "TEXT"),
    ] {
        let exists: (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM pragma_table_info('directories') WHERE name='{}'",
            col
        ))
        .fetch_one(&mut *tx)
        .await?;
        if exists.0 == 0 {
            sqlx::query(&format!("ALTER TABLE directories ADD COLUMN {} {}", col, def))
                .execute(&mut *tx)
                .await?;
        }
    }

    // 2) 清理旧两级数据：无 directory_id 的文件（旧 package_id>0 两级文件）
    let purged: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE directory_id IS NULL")
            .fetch_one(&mut *tx)
            .await?;
    log::info!("[db] 0006 清理旧两级文件（directory_id IS NULL）共 {} 条", purged);
    sqlx::query("DELETE FROM files WHERE directory_id IS NULL")
        .execute(&mut *tx)
        .await?;

    // 3) 重建 files 表：去掉 package_id 列与外键，directory_id NOT NULL
    sqlx::query(
        "CREATE TABLE files_new (
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
        )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO files_new
            (id, rel_path, name, ext, kind, bytes, width, height,
             frame_count, modified_at, content_hash, deleted, directory_id)
         SELECT id, rel_path, name, ext, kind, bytes, width, height,
                frame_count, modified_at, content_hash, deleted, directory_id
         FROM files",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DROP TABLE files").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE files_new RENAME TO files").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_directory_id ON files(directory_id)")
        .execute(&mut *tx)
        .await?;

    // 4) 重建 selections 表：package_id → directory_id，scope 'package' → 'directory'
    sqlx::query(
        "CREATE TABLE selections_new (
            id           INTEGER PRIMARY KEY,
            scope        TEXT NOT NULL CHECK(scope IN ('directory','file','exclude')),
            directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE,
            file_id      INTEGER REFERENCES files(id) ON DELETE CASCADE,
            project_id   INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            created_at   INTEGER NOT NULL,
            CHECK((scope='directory' AND directory_id IS NOT NULL) OR
                  (scope IN ('file','exclude') AND file_id IS NOT NULL))
        )",
    )
    .execute(&mut *tx)
    .await?;
    // 旧 selections 数据全是两级视图产物，直接丢弃（scope='package' 在新表无对应）
    // 不迁移，新表从空开始
    sqlx::query("DROP TABLE selections").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE selections_new RENAME TO selections").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_selections_project ON selections(project_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_selections_directory ON selections(directory_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_selections_file ON selections(file_id)")
        .execute(&mut *tx)
        .await?;

    // 5) DROP 两级视图表 + 去重表
    for table in [
        "duplicate_members",
        "duplicate_groups",
        "dismissed_pairs",
        "packages",
        "categories",
    ] {
        // 临时关闭外键检查，避免残留外键依赖阻止 drop
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *tx)
            .await?;
        sqlx::query(&format!("DROP TABLE IF EXISTS {}", table))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("PRAGMA foreign_keys=ON").execute(&mut *tx).await?;

    tx.commit().await?;
    // 记录迁移已应用（幂等标记）
    sqlx::query("INSERT OR IGNORE INTO schema_migrations(version) VALUES (6)")
        .execute(pool)
        .await?;
    log::info!("[db] 0006 迁移完成：两级视图已移除，目录树架构生效");
    Ok(())
}
