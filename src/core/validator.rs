const MAX_CONTENT_LENGTH: usize = 5000;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("内容超出最大长度限制（{MAX_CONTENT_LENGTH} 字符），当前长度: {0}")]
    ContentTooLong(usize),

    #[error("任务名称不能为空")]
    NameEmpty,

    #[error("内容不能为空")]
    ContentEmpty,
}

pub fn validate_content(content: &str, name: &str) -> Result<(), ValidationError> {
    if name.trim().is_empty() {
        return Err(ValidationError::NameEmpty);
    }
    if content.trim().is_empty() {
        return Err(ValidationError::ContentEmpty);
    }
    if content.len() > MAX_CONTENT_LENGTH {
        return Err(ValidationError::ContentTooLong(content.len()));
    }
    Ok(())
}
