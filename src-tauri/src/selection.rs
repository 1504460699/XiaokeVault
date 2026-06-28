use crate::error::AppError;
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub export_root: String,
}

#[derive(Debug, Serialize)]
pub struct SelectionSummary {
    pub directory_count: i64,
    pub file_count: i64,
    pub total_bytes: i64,
}

#[tauri::command]
pub async fn create_project(
    name: String,
    export_root: String,
    pool: State<'_, SqlitePool>,
) -> Result<Project, AppError> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO projects(name,export_root,created_at) VALUES(?,?,?)")
        .bind(&name)
        .bind(&export_root)
        .bind(now)
        .execute(&*pool)
        .await?;
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM projects WHERE name=? AND export_root=? ORDER BY id DESC")
        .bind(&name)
        .bind(&export_root)
        .fetch_one(&*pool)
        .await?;
    Ok(Project {
        id,
        name,
        export_root,
    })
}

#[tauri::command]
pub async fn list_projects(pool: State<'_, SqlitePool>) -> Result<Vec<Project>, AppError> {
    let rows: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id,name,export_root FROM projects ORDER BY id DESC")
            .fetch_all(&*pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, export_root)| Project {
            id,
            name,
            export_root,
        })
        .collect())
}

/// 设置勾选：scope/directory_id/file_id/action('add'|'remove')
/// scope: 'directory'（整目录含子树）| 'file'（单文件）| 'exclude'（排除）
#[tauri::command]
pub async fn set_selection(
    project_id: i64,
    scope: String,
    directory_id: Option<i64>,
    file_id: Option<i64>,
    action: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    if action == "add" {
        sqlx::query("INSERT INTO selections(project_id,scope,directory_id,file_id,created_at) VALUES(?,?,?,?,?)")
            .bind(project_id)
            .bind(&scope)
            .bind(directory_id)
            .bind(file_id)
            .bind(chrono::Utc::now().timestamp())
            .execute(&*pool)
            .await?;
    } else {
        match scope.as_str() {
            "directory" => {
                sqlx::query("DELETE FROM selections WHERE project_id=? AND scope='directory' AND directory_id=?")
                    .bind(project_id)
                    .bind(directory_id)
                    .execute(&*pool)
                    .await?;
            }
            _ => {
                sqlx::query("DELETE FROM selections WHERE project_id=? AND scope=? AND file_id=?")
                    .bind(project_id)
                    .bind(&scope)
                    .bind(file_id)
                    .execute(&*pool)
                    .await?;
            }
        }
    }
    Ok(())
}

/// 清空项目的所有勾选
#[tauri::command]
pub async fn clear_selections(project_id: i64, pool: State<'_, SqlitePool>) -> Result<(), AppError> {
    sqlx::query("DELETE FROM selections WHERE project_id=?")
        .bind(project_id)
        .execute(&*pool)
        .await?;
    Ok(())
}

/// 查某目录子树内已被单文件勾选的 file_id 列表（进目录时回填 UI 用）。
/// 用递归 CTE 取 dir_id 及其所有子孙目录下的文件。
#[tauri::command]
pub async fn get_selected_file_ids(
    project_id: i64,
    dir_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<i64>, AppError> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT file_id FROM selections
         WHERE project_id=? AND scope='file' AND file_id IS NOT NULL
           AND file_id IN (
             SELECT f.id FROM files f
             JOIN (
               WITH RECURSIVE subtree(id) AS (
                 SELECT id FROM directories WHERE id=?
                 UNION ALL
                 SELECT d.id FROM directories d JOIN subtree s ON d.parent_id=s.id
               )
               SELECT id FROM subtree
             ) AS sub ON f.directory_id=sub.id
           )",
    )
    .bind(project_id)
    .bind(dir_id)
    .fetch_all(&*pool)
    .await?;
    Ok(ids)
}

/// 查某目录是否已被整体勾选（'all'）或仅部分文件勾选（'partial'）或未勾（'none'）
#[tauri::command]
pub async fn get_directory_selection_state(
    project_id: i64,
    dir_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    // 是否整体勾选了该目录
    let dir_selected: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM selections WHERE project_id=? AND scope='directory' AND directory_id=? LIMIT 1",
    )
    .bind(project_id)
    .bind(dir_id)
    .fetch_optional(&*pool)
    .await?;
    if dir_selected.is_some() {
        return Ok("all".to_string());
    }
    // 是否有该子树内的单文件勾选
    let sel_files: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM selections s
         WHERE s.project_id=? AND s.scope='file'
           AND s.file_id IN (
             SELECT f.id FROM files f
             JOIN (
               WITH RECURSIVE subtree(id) AS (
                 SELECT id FROM directories WHERE id=?
                 UNION ALL
                 SELECT d.id FROM directories d JOIN subtree s2 ON d.parent_id=s2.id
               )
               SELECT id FROM subtree
             ) AS sub ON f.directory_id=sub.id
           )",
    )
    .bind(project_id)
    .bind(dir_id)
    .fetch_one(&*pool)
    .await?;
    Ok(if sel_files > 0 {
        "partial".to_string()
    } else {
        "none".to_string()
    })
}

/// 统计整个项目的勾选汇总。
/// directory_count=整体勾选的目录数（去重）；file_count/total_bytes 为实际命中的文件数与字节数。
#[tauri::command]
pub async fn get_selection_summary(
    project_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<SelectionSummary, AppError> {
    // 整体勾选的目录数
    let (dir_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT directory_id) FROM selections WHERE project_id=? AND scope='directory'",
    )
    .bind(project_id)
    .fetch_one(&*pool)
    .await?;

    // 实际命中的文件：整体目录勾选覆盖的子树文件 ∪ 单文件勾选，再排除 exclude
    let (file_count, total_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(f.bytes),0) FROM files f
         WHERE f.deleted=0 AND (
           f.directory_id IN (
             SELECT sub.id FROM selections sel
             JOIN (
               WITH RECURSIVE subtree(id) AS (
                 SELECT id FROM directories WHERE id IN (
                   SELECT directory_id FROM selections WHERE project_id=? AND scope='directory' AND directory_id IS NOT NULL
                 )
                 UNION ALL
                 SELECT d.id FROM directories d JOIN subtree s ON d.parent_id=s.id
               )
               SELECT id FROM subtree
             ) AS sub
           )
           OR f.id IN (SELECT file_id FROM selections WHERE project_id=? AND scope='file')
         ) AND f.id NOT IN (SELECT file_id FROM selections WHERE project_id=? AND scope='exclude')",
    )
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .fetch_one(&*pool)
    .await?;
    Ok(SelectionSummary {
        directory_count: dir_count,
        file_count,
        total_bytes,
    })
}
