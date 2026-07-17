use clap::Args;
use crate::cli::CliError;
use crate::core::pusher;
use crate::core::validator;
use crate::config;
use crate::storage::history;

#[derive(Args, Debug)]
pub struct PushArgs {
    /// 任务名称
    #[arg(short, long)]
    pub name: String,

    /// Markdown 文件路径
    #[arg(short, long)]
    pub file: Option<String>,

    /// 模板名称
    #[arg(short, long)]
    pub template: Option<String>,

    /// 模板变量（key=value）
    #[arg(long)]
    pub var: Vec<String>,

    /// 执行结果描述
    #[arg(short, long)]
    pub result: Option<String>,

    /// 周期任务 ID
    #[arg(short, long)]
    pub schedule_id: Option<String>,

    /// 试运行（仅校验，不推送）
    #[arg(long)]
    pub dry_run: bool,
}

pub fn execute(args: PushArgs) -> Result<(), CliError> {
    let cfg = config::profile::load()?;

    // 1. Read content from file, template, or stdin
    let content = if let Some(ref path) = args.file {
        std::fs::read_to_string(path)
            .map_err(|e| CliError::Push(format!("读取文件失败: {e}")))?

    } else if let Some(ref tmpl_name) = args.template {
        let vars = crate::template::manager::parse_vars(&args.var);
        crate::template::manager::render(tmpl_name, &vars)
            .map_err(|e| CliError::Template(e.to_string()))?
    } else {
        // Read from stdin
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| CliError::Push(format!("读取标准输入失败: {e}")))?;
        buf
    };

    if content.trim().is_empty() {
        return Err(CliError::Push("内容为空".into()));
    }

    // 2. Validate content
    validator::validate_content(&content, &args.name)
        .map_err(|e| CliError::Push(e.to_string()))?;

    // 3. Get auth code
    let auth_code: String = match config::keychain::get_auth_code() {
        Ok(code) => code,
        Err(_) => match std::env::var("HWPUSH_AUTH_CODE") {
            Ok(code) => code,
            Err(_) => {
                return Err(CliError::Push(
                    "未找到认证码。请运行 `hwpush config auth` 或设置 HWPUSH_AUTH_CODE 环境变量。"
                        .into(),
                ));
            }
        },
    };

    // 4. Build payload and push
    let result = args.result.unwrap_or_else(|| cfg.defaults.result.clone());
    let payload = pusher::build_payload(&auth_code, &args.name, &content, &result, &args.schedule_id);

    if args.dry_run {
        println!("--- 试运行 ---");
        println!(
            "负载内容:\n{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        println!("--- 试运行结束 ---");
        return Ok(());
    }

    // 5. Execute push
    let response = pusher::push(&cfg.push.service_url, &payload, cfg.push.timeout_secs)
        .map_err(|e| CliError::Push(e.to_string()))?;

    // 6. Record to history
    let _ = history::record_push(
        &cfg.storage.history_db_path,
        &args.name,
        &args.schedule_id.clone().unwrap_or_default(),
        &response.code,
    );

    // 7. Output result
    if response.success() {
        println!("✅ API 已接受推送请求（状态码: {}）", response.code);
        println!("");
        println!("   📱 如果设备未收到通知，请检查：");
        println!("      1. 确保已登录华为账号");
        println!("      2. 打开负一屏 → 头像 → 我的 → 动态管理");
        println!("      3. 确保「AI 任务完成通知」开关已开启");
        println!("      4. 确保设备网络连接正常");
    } else {
        println!("❌ 推送失败！");
        println!("   状态码: {}", response.code);
        println!("   错误信息: {}", response.message);
        return Err(CliError::Push(response.message));
    }

    Ok(())
}
