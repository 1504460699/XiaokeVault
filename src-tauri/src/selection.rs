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
pub struct PackageSelectionState {
    pub package_id: i64,
    /// "all"=整包勾, "partial"=部分文件勾, "none"=未勾
    pub state: String,
    pub file_count: i64,
    pub selected_files: i64,
}

#[derive(Debug, Serialize)]
pub struct SelectionSummary {
    pub package_count: i64,
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
        .await
        ?;
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM projects WHERE name=? AND export_root=? ORDER BY id DESC")
        .bind(&name)
        .bind(&export_root)
        .fetch_one(&*pool)
        .await
        ?;
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
            .await
            ?;
    Ok(rows
        .into_iter()
        .map(|(id, name, export_root)| Project {
            id,
            name,
            export_root,
        })
        .collect())
}

/// 设置勾选：scope/package_id/file_id/action('add'|'remove')
#[tauri::command]
pub async fn set_selection(
    project_id: i64,
    scope: String,
    package_id: Option<i64>,
    file_id: Option<i64>,
    action: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    if action == "add" {
        sqlx::query("INSERT INTO selections(project_id,scope,package_id,file_id,created_at) VALUES(?,?,?,?,?)")
            .bind(project_id)
            .bind(&scope)
            .bind(package_id)
            .bind(file_id)
            .bind(chrono::Utc::now().timestamp())
            .execute(&*pool)
            .await
            ?;
    } else {
        match scope.as_str() {
            "package" => {
                sqlx::query("DELETE FROM selections WHERE project_id=? AND scope='package' AND package_id=?")
                    .bind(project_id)
                    .bind(package_id)
                    .execute(&*pool)
                    .await
                    ?;
            }
            _ => {
                sqlx::query("DELETE FROM selections WHERE project_id=? AND scope=? AND file_id=?")
                    .bind(project_id)
                    .bind(&scope)
                    .bind(file_id)
                    .execute(&*pool)
                    .await
                    ?;
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
        .await
        ?;
    Ok(())
}

/// 查某包内已被单文件勾选的 file_id 列表（进包时回填 UI 用）
#[tauri::command]
pub async fn get_selected_file_ids(
    project_id: i64,
    pkg_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<i64>, AppError> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT file_id FROM selections
         WHERE project_id=? AND scope='file' AND file_id IS NOT NULL
           AND file_id IN (SELECT id FROM files WHERE package_id=?)",
    )
    .bind(project_id)
    .bind(pkg_id)
    .fetch_all(&*pool)
    .await
    ?;
    Ok(ids)
}

/// 查某分类下各包的勾选状态
#[tauri::command]
pub async fn get_category_selection_states(
    project_id: i64,
    category_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<PackageSelectionState>, AppError> {
    let pkgs: Vec<(i64, i64)> =
        sqlx::query_as("SELECT id, file_count FROM packages WHERE category_id=? ORDER BY name")
            .bind(category_id)
            .fetch_all(&*pool)
            .await
            ?;
    let mut out = Vec::new();
    for (pkg_id, file_count) in pkgs {
        let pkg_selected: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM selections WHERE project_id=? AND scope='package' AND package_id=? LIMIT 1",
        )
        .bind(project_id)
        .bind(pkg_id)
        .fetch_optional(&*pool)
        .await
        ?;
        let sel_files: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM selections WHERE project_id=? AND scope='file' AND file_id IN (SELECT id FROM files WHERE package_id=?)",
        )
        .bind(project_id)
        .bind(pkg_id)
        .fetch_one(&*pool)
        .await
        ?;
        let state = if pkg_selected.is_some() {
            "all"
        } else if sel_files > 0 {
            "partial"
        } else {
            "none"
        };
        out.push(PackageSelectionState {
            package_id: pkg_id,
            state: state.to_string(),
            file_count,
            selected_files: sel_files,
        });
    }
    Ok(out)
}

/// 统计整个项目的勾选汇总
#[tauri::command]
pub async fn get_selection_summary(
    project_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<SelectionSummary, AppError> {
    let (pkg_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT package_id) FROM selections WHERE project_id=? AND scope='package'",
    )
    .bind(project_id)
    .fetch_one(&*pool)
    .await
    ?;
    let (file_count, total_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(f.bytes),0) FROM files f
         WHERE f.deleted=0 AND (
           f.package_id IN (SELECT package_id FROM selections WHERE project_id=? AND scope='package')
           OR f.id IN (SELECT file_id FROM selections WHERE project_id=? AND scope='file')
         ) AND f.id NOT IN (SELECT file_id FROM selections WHERE project_id=? AND scope='exclude')",
    )
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .fetch_one(&*pool)
    .await
    ?;
    Ok(SelectionSummary {
        package_count: pkg_count,
        file_count,
        total_bytes,
    })
}
