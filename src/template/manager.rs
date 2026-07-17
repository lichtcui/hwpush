use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::profile;

pub struct TemplateInfo {
    pub name: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub path: PathBuf,
}

pub fn list() -> Result<Vec<TemplateInfo>, String> {
    let mut templates = Vec::new();

    // List user templates
    let user_dir = profile::default_template_dir();
    if user_dir.exists() {
        let entries = std::fs::read_dir(&user_dir)
            .map_err(|e| format!("读取模板目录失败: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let name = path.file_stem().unwrap().to_string_lossy().into();
                let desc = parse_front_matter(&path).ok().flatten();
                templates.push(TemplateInfo {
                    name,
                    description: desc,
                    path,
                });
            }
        }
    }

    // List built-in templates
    let builtin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
    if builtin_dir.exists() {
        let entries = std::fs::read_dir(&builtin_dir)
            .map_err(|e| format!("读取内置模板目录失败: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let name = path.file_stem().unwrap().to_string_lossy().into();
                // Skip if user has overridden
                if templates.iter().any(|t| t.name == name) {
                    continue;
                }
                let desc = parse_front_matter(&path).ok().flatten();
                templates.push(TemplateInfo {
                    name,
                    description: desc,
                    path,
                });
            }
        }
    }

    Ok(templates)
}

pub fn resolve_path(name: &str) -> Option<PathBuf> {
    // Check user templates first
    let user_path = profile::default_template_dir().join(format!("{name}.md"));
    if user_path.exists() {
        return Some(user_path);
    }

    // Check built-in templates
    let builtin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates")
        .join(format!("{name}.md"));
    if builtin_path.exists() {
        return Some(builtin_path);
    }

    None
}

pub fn read_raw(name: &str) -> Result<String, String> {
    let path =
        resolve_path(name).ok_or_else(|| format!("模板 '{name}' 未找到"))?;
    std::fs::read_to_string(&path).map_err(|e| format!("读取模板失败: {e}"))
}

pub fn render(name: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    let content = read_raw(name)?;

    // Strip front matter
    let body = strip_front_matter(&content);

    // Simple variable interpolation: {{key}} -> value
    let rendered = interpolate(body, vars);
    Ok(rendered)
}

pub fn create(name: &str) -> Result<(), String> {
    let dir = profile::default_template_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建模板目录失败: {e}"))?;

    let path = dir.join(format!("{name}.md"));
    if path.exists() {
        return Err(format!("模板 '{name}' 已存在"));
    }

    let default_content = format!(
        r#"---
name: {name}
description: ""
variables:
  - name: content
    description: 主要内容
    required: true
---

# {{{{name}}}} 任务报告

{{{{content}}}}

---

*生成时间: {{{{date}}}} {{{{time}}}}*
"#
    );

    std::fs::write(&path, default_content)
        .map_err(|e| format!("写入模板失败: {e}"))?;

    Ok(())
}

pub fn delete(name: &str) -> Result<(), String> {
    let path = resolve_path(name)
        .ok_or_else(|| format!("模板 '{name}' 未找到"))?;
    std::fs::remove_file(&path)
        .map_err(|e| format!("删除模板失败: {e}"))
}

pub fn parse_vars(var_args: &[String]) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for arg in var_args {
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            vars.insert(key.to_string(), value.to_string());
        }
    }

    // Add default date/time variables
    let now = chrono::Local::now();
    vars.entry("date".into())
        .or_insert_with(|| now.format("%Y-%m-%d").to_string());
    vars.entry("time".into())
        .or_insert_with(|| now.format("%H:%M:%S").to_string());

    vars
}

fn parse_front_matter(path: &PathBuf) -> Result<Option<String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {e}"))?;

    if let Some(body) = content.strip_prefix("---") {
        if let Some(end) = body.find("---") {
            let front = &body[..end];
            for line in front.lines() {
                if let Some(desc_val) = line.strip_prefix("description:") {
                    let desc = desc_val.trim().trim_matches('"').to_string();
                    return if desc.is_empty() { Ok(None) } else { Ok(Some(desc)) };
                }
            }
        }
    }
    Ok(None)
}

fn strip_front_matter(content: &str) -> &str {
    if let Some(body) = content.strip_prefix("---") {
        if let Some(end) = body.find("---") {
            return body[end + 3..].trim();
        }
    }
    content
}

fn interpolate(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}
