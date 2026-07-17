use clap::Subcommand;
use thiserror::Error;
use crate::config;

mod init;
mod push;
mod template;

pub use init::*;
pub use push::*;
pub use template::*;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// 初始化 hiboard 配置
    Init(InitArgs),

    /// 推送任务结果到负一屏
    Push(PushArgs),

    /// 管理模板
    Template(TemplateArgs),

    /// 管理配置
    Config(ConfigArgs),
}

#[derive(clap::Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 查看当前配置
    Get,
    /// 设置配置键值对
    Set { key: String, value: String },
    /// 更新 Keychain 认证码
    Auth,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("推送错误: {0}")]
    Push(String),

    #[error("模板错误: {0}")]
    Template(String),

    #[error("Keychain 错误: {0}")]
    Keychain(String),
}

pub fn dispatch(command: Command) -> Result<(), CliError> {
    match command {
        Command::Init(args) => init::execute(args),
        Command::Push(args) => push::execute(args),
        Command::Template(args) => template::execute(args),
        Command::Config(args) => execute_config(args),
    }
}

fn execute_config(args: ConfigArgs) -> Result<(), CliError> {
    match args.action {
        ConfigAction::Get => {
            let cfg = config::profile::load()?;
            println!("{}", toml::to_string_pretty(&cfg).unwrap_or_default());
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = config::profile::load()?;
            config::profile::set_value(&mut cfg, &key, &value);
            config::profile::save(&cfg)?;
            println!("配置已更新: {key} = {value}");
            Ok(())
        }
        ConfigAction::Auth => {
            let code = rpassword::prompt_password("请输入认证码: ")
                .map_err(|e| CliError::Config(e.to_string()))?;
            config::keychain::set_auth_code(&code)
                .map_err(|e| CliError::Keychain(e.to_string()))?;
            println!("认证码已更新到 Keychain。");
            Ok(())
        }
    }
}
