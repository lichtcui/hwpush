use clap::Args;
use crate::cli::CliError;
use crate::config;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// 认证码（提供后跳过交互式输入）
    #[arg(short, long)]
    pub code: Option<String>,
}

pub fn execute(args: InitArgs) -> Result<(), CliError> {
    // 1. Create default config if not exists
    let config_path = config::profile::default_config_path();
    if config_path.exists() {
        println!("配置文件已存在: {}", config_path.display());
    } else {
        let cfg = config::profile::Config::default();
        config::profile::save(&cfg).map_err(|e| CliError::Config(e.to_string()))?;
        println!("已创建默认配置文件: {}", config_path.display());
    }

    // 2. Auth code: use --code flag or prompt interactively
    if let Some(code) = &args.code {
        config::keychain::set_auth_code(code)
            .map_err(|e| CliError::Keychain(e.to_string()))?;
        println!("认证码已保存到 Keychain。");
    } else {
        let code = rpassword::prompt_password("请输入认证码（直接回车跳过）: ")
            .map_err(|e| CliError::Config(e.to_string()))?;
        if !code.is_empty() {
            config::keychain::set_auth_code(&code)
                .map_err(|e| CliError::Keychain(e.to_string()))?;
            println!("认证码已保存到 Keychain。");
        } else {
            println!("已跳过认证码设置。之后可通过 `hwpush config auth` 配置。");
        }
    }

    // 3. Create default templates directory
    let template_dir = config::profile::default_template_dir();
    std::fs::create_dir_all(&template_dir)
        .map_err(|e| CliError::Config(format!("创建模板目录失败: {e}")))?;
    println!("模板目录已就绪: {}", template_dir.display());

    // 4. Ensure storage directory exists
    let storage_dir = config::profile::default_storage_dir();
    std::fs::create_dir_all(&storage_dir)
        .map_err(|e| CliError::Config(format!("创建存储目录失败: {e}")))?;
    println!("存储目录已就绪: {}", storage_dir.display());

    println!("\n✅ hwpush 初始化成功！");
    Ok(())
}
