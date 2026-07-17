use clap::{Args, Subcommand};
use crate::cli::CliError;
use crate::template;

#[derive(Args, Debug)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub action: TemplateAction,
}

#[derive(Subcommand, Debug)]
pub enum TemplateAction {
    /// 列出所有模板
    List,
    /// 查看模板内容
    Show { name: String },
    /// 创建新模板
    New { name: String },
    /// 使用 $EDITOR 编辑模板
    Edit { name: String },
    /// 删除模板
    Delete { name: String },
}

pub fn execute(args: TemplateArgs) -> Result<(), CliError> {
    match args.action {
        TemplateAction::List => {
            let list = template::manager::list()
                .map_err(|e| CliError::Template(e.to_string()))?;
            if list.is_empty() {
                println!("未找到模板。");
                return Ok(());
            }
            println!("可用模板:");
            for tmpl in &list {
                println!("  - {}: {}", tmpl.name, tmpl.description.as_deref().unwrap_or(""));
            }
        }
        TemplateAction::Show { name } => {
            let content = template::manager::read_raw(&name)
                .map_err(|e| CliError::Template(e.to_string()))?;
            println!("{content}");
        }
        TemplateAction::New { name } => {
            template::manager::create(&name)
                .map_err(|e| CliError::Template(e.to_string()))?;
            println!("模板 '{name}' 已创建。");
        }
        TemplateAction::Edit { name } => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
            let path = template::manager::resolve_path(&name)
                .ok_or_else(|| CliError::Template(format!("模板 '{name}' 未找到")))?;
            let status = std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|e| CliError::Template(format!("启动编辑器失败: {e}")))?;
            if !status.success() {
                return Err(CliError::Template("编辑器异常退出".into()));
            }
        }
        TemplateAction::Delete { name } => {
            template::manager::delete(&name)
                .map_err(|e| CliError::Template(e.to_string()))?;
            println!("模板 '{name}' 已删除。");
        }
    }
    Ok(())
}
