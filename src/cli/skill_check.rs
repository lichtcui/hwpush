use clap::Args;
use serde_json::json;

use crate::cli::CliError;
use crate::config;
use crate::core::skill_check;

#[derive(Args, Debug)]
pub struct SkillCheckArgs {
    /// ClawHub 仓库地址（可用环境变量 CLAWHUB_REGISTRY 覆盖）
    #[arg(long, default_value_t = default_registry())]
    pub registry: String,

    /// 已同步基线版本（默认取配置 [skill].synced_version）
    #[arg(long)]
    pub synced_version: Option<String>,

    /// 检查后将最新版本记录为已同步版本（写入配置）
    #[arg(long)]
    pub mark_synced: bool,

    /// JSON 格式输出（便于 AI 和脚本调用）
    #[arg(short, long)]
    pub json: bool,
}

fn default_registry() -> String {
    std::env::var("CLAWHUB_REGISTRY").unwrap_or_else(|_| skill_check::DEFAULT_REGISTRY.into())
}

/// 拉取最新版 skill 的关键文件并计算兼容性；任一文件拉取失败则返回 None（降级为纯版本提示）
fn fetch_compatibility(
    registry: &str,
    timeout_secs: u64,
) -> Result<Option<skill_check::Compatibility>, String> {
    let config_json = match skill_check::fetch_file(registry, "config.json", timeout_secs) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let task_pusher_py =
        match skill_check::fetch_file(registry, "scripts/task_pusher.py", timeout_secs) {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };
    Ok(Some(skill_check::check_compatibility(
        &config_json,
        &task_pusher_py,
    )))
}

pub fn execute(args: SkillCheckArgs) -> Result<(), CliError> {
    let mut cfg = config::profile::load()?;
    let baseline = args
        .synced_version
        .clone()
        .unwrap_or_else(|| cfg.skill.synced_version.clone());

    // 1. 拉取版本列表
    let versions = match skill_check::fetch_versions(&args.registry, cfg.push.timeout_secs) {
        Ok(v) => v,
        Err(e) => {
            if args.json {
                println!(
                    "{}",
                    json!({
                        "skill": skill_check::SKILL_SLUG,
                        "registry": args.registry,
                        "synced_version": baseline,
                        "has_update": false,
                        "error": format!("检查更新失败: {e}"),
                    })
                );
            }
            return Err(CliError::SkillCheck(format!("检查更新失败: {e}")));
        }
    };

    let latest = versions
        .first()
        .ok_or_else(|| CliError::SkillCheck("仓库未返回任何版本信息".into()))?;

    // 2. 对比基线版本
    let newer = skill_check::newer_versions(&baseline, &versions);
    let has_update = !newer.is_empty();

    // 3. 有新版本时，拉取最新版关键文件做格式兼容性检查
    let compatibility = if has_update {
        match fetch_compatibility(&args.registry, cfg.push.timeout_secs) {
            Ok(compat) => compat,
            Err(e) => return Err(CliError::SkillCheck(format!("检查兼容性失败: {e}"))),
        }
    } else {
        None
    };
    let compatible = compatibility.as_ref().is_some_and(|c| c.is_compatible());

    // 4. 可选：记录已同步版本
    if args.mark_synced {
        cfg.skill.synced_version = latest.version.clone();
        config::profile::save(&cfg)?;
    }

    // 5. 输出结果
    if args.json {
        let new_versions: Vec<_> = newer
            .iter()
            .map(|v| {
                json!({
                    "version": v.version,
                    "changelog": v.changelog,
                    "changelog_source": v.changelog_source,
                })
            })
            .collect();
        let diff: Option<serde_json::Value> = compatibility
            .as_ref()
            .map(|c| serde_json::to_value(c).unwrap_or_default());
        println!(
            "{}",
            json!({
                "skill": skill_check::SKILL_SLUG,
                "registry": args.registry,
                "synced_version": baseline,
                "latest_version": latest.version,
                "has_update": has_update,
                "compatible": if has_update { Some(compatible) } else { None },
                "diff": diff,
                "mark_synced": args.mark_synced,
                "new_versions": new_versions,
            })
        );
    } else {
        println!(
            "🔍 检查 {} 更新（仓库: {}）",
            skill_check::SKILL_SLUG,
            args.registry
        );
        println!("   当前同步版本: v{baseline}");
        println!("   仓库最新版本: v{}", latest.version);

        if has_update {
            match &compatibility {
                // 兼容：无需修改代码，一行提示即可（不列 changelog，避免噪音）
                Some(c) if c.is_compatible() => {
                    println!(
                        "✅ 有更新（v{baseline} → v{}），但负载格式与 hwpush 完全兼容，无需修改代码",
                        latest.version
                    );
                }
                // 不兼容或无法验证：列出差异与更新内容
                Some(c) => {
                    println!("🚨 检测到需要关注的变更：v{baseline} → v{}", latest.version);
                    println!();
                    for diff in c.diffs() {
                        let (status, label) = match diff.matched {
                            Some(false) => ("⚠️", "不一致"),
                            None => ("❓", "无法验证"),
                            Some(true) => unreachable!("diff 中不应包含匹配项"),
                        };
                        println!("{status} {label}：{}", diff.key);
                        println!("     skill 最新版: {}", diff.skill_value);
                        println!("     hwpush 当前值: {}", diff.hwpush_value);
                    }
                    println!();
                    for v in &newer {
                        let changelog = v.changelog.trim();
                        println!("📦 v{}", v.version);
                        println!("{}", "-".repeat(30));
                        if changelog.is_empty() {
                            println!("（无更新说明）");
                        } else {
                            println!("{changelog}");
                        }
                        println!();
                    }
                    println!(
                        "提示：修改 hwpush 实现后，可运行 `hwpush skill-check --mark-synced` 记录已同步版本"
                    );
                }
                // 文件拉取失败，降级为纯版本提示
                None => {
                    println!(
                        "ℹ️ 有更新（v{baseline} → v{}），但无法验证格式兼容性（拉取文件失败），请手动核对",
                        latest.version
                    );
                    println!();
                    for v in &newer {
                        let changelog = v.changelog.trim();
                        println!("📦 v{}", v.version);
                        println!("{}", "-".repeat(30));
                        if changelog.is_empty() {
                            println!("（无更新说明）");
                        } else {
                            println!("{changelog}");
                        }
                        println!();
                    }
                }
            }
        } else {
            println!("✅ 已是最新版本，无更新");
        }
        if args.mark_synced {
            println!("已记录已同步版本：v{}（写入配置）", latest.version);
        }
    }

    Ok(())
}
