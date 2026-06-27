use serde::Serialize;

/// 应用统一错误类型。每个 variant 的 Display 是面向用户的中文提示。
/// 前端通过 Serialize 拿到 { code, message }，可按 code 分支处理。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 数据库错误（sqlx）。最常见，提示用户稍后重试。
    #[error("数据库错误：{0}")]
    Database(#[from] sqlx::Error),

    /// 找不到资源（库/包/文件等）。前端可静默或提示。
    #[error("找不到{what}")]
    NotFound { what: String },

    /// 用户输入不合法（如必填项为空、格式错）。保留原始中文消息。
    #[error("{0}")]
    InvalidInput(String),

    /// 文件系统操作失败（读写/移动/创建目录）。
    #[error("文件操作失败：{0}")]
    Io(#[from] std::io::Error),

    /// 图片处理失败（缩略图生成等）。
    #[error("图片处理失败：{0}")]
    Image(#[from] image::ImageError),

    /// 序列化/反序列化失败。
    #[error("数据格式错误：{0}")]
    Serde(#[from] serde_json::Error),

    /// zip 压缩/解压失败。
    #[error("压缩包处理失败：{0}")]
    Zip(#[from] zip::result::ZipError),

    /// 其他内部错误（兜底）。
    #[error("内部错误：{0}")]
    Internal(String),
}

/// 序列化成 { code, message } 结构，前端据此分支。
/// code 是变体名（如 "Database"/"NotFound"），message 是中文 Display。
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let code = match self {
            AppError::Database(_) => "Database",
            AppError::NotFound { .. } => "NotFound",
            AppError::InvalidInput(_) => "InvalidInput",
            AppError::Io(_) => "Io",
            AppError::Image(_) => "Image",
            AppError::Serde(_) => "Serde",
            AppError::Zip(_) => "Zip",
            AppError::Internal(_) => "Internal",
        };
        let mut st = serializer.serialize_struct("AppError", 2)?;
        st.serialize_field("code", code)?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

/// 便捷：把任意字符串错误包成 Internal。
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
