//! today-task skill 更新检查
//!
//! 从 ClawHub 仓库查询 `today-task` skill 的版本列表与更新日志（changelog），
//! 与 hwpush 已同步的版本基线对比，判断参考实现是否有新版本，以及新增了什么内容。

use std::cmp::Ordering;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// 查询的 skill 名称（华为负一屏参考实现）
pub const SKILL_SLUG: &str = "today-task";

/// 默认 ClawHub 仓库地址（官方站点 API 提供 changelog）
pub const DEFAULT_REGISTRY: &str = "https://clawhub.com";

/// ClawHub versions 接口返回的单条版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    /// 版本号，如 `1.0.17`
    pub version: String,
    /// 该版本的更新日志（changelog）
    #[serde(default)]
    pub changelog: String,
    /// 更新日志来源（user / auto）
    #[serde(default, rename = "changelogSource")]
    pub changelog_source: String,
}

/// ClawHub versions 接口响应
#[derive(Debug, Deserialize)]
pub struct VersionsResponse {
    /// 版本列表（从新到旧）
    pub items: Vec<SkillVersion>,
}

/// 解析版本号 `1.0.17` → `[1, 0, 17]`；支持 `v1.0.17` 前缀
pub fn parse_version(v: &str) -> Option<Vec<u64>> {
    let s = v.trim().trim_start_matches(['v', 'V']);
    if s.is_empty() {
        return None;
    }
    let mut nums = Vec::new();
    for part in s.split('.') {
        nums.push(part.parse::<u64>().ok()?);
    }
    Some(nums)
}

/// 比较两个版本号，返回 `a` 与 `b` 的大小关系；版本号非法时返回 `None`
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let a = parse_version(a)?;
    let b = parse_version(b)?;
    let len = a.len().max(b.len());
    for i in 0..len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av != bv {
            return Some(av.cmp(&bv));
        }
    }
    Some(Ordering::Equal)
}

/// 判断 `version` 是否严格新于 `baseline`
pub fn is_newer_than(version: &str, baseline: &str) -> bool {
    matches!(compare_versions(version, baseline), Some(Ordering::Greater))
}

/// 返回严格新于基线的版本列表（保持接口返回的从新到旧顺序）
pub fn newer_versions<'a>(baseline: &str, versions: &'a [SkillVersion]) -> Vec<&'a SkillVersion> {
    versions
        .iter()
        .filter(|v| is_newer_than(&v.version, baseline))
        .collect()
}

/// 拉取 skill 的版本列表（从新到旧）
pub fn fetch_versions(registry: &str, timeout_secs: u64) -> Result<Vec<SkillVersion>, String> {
    let base = registry.trim_end_matches('/');
    let url = format!("{base}/api/v1/skills/{SKILL_SLUG}/versions?limit=50");

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("网络错误: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}（仓库可能不支持该接口）", resp.status()));
    }

    let body = resp.text().map_err(|e| format!("读取响应体失败: {e}"))?;

    let parsed: VersionsResponse = serde_json::from_str(&body)
        .map_err(|e| format!("解析版本列表失败: {e}（响应体: {}）", truncate(&body, 200)))?;

    Ok(parsed.items)
}

/// 截断长字符串用于错误信息
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

// ---------------------------------------------------------------------------
// 兼容性检查：对比最新版 skill 的负载格式与 hwpush 的实现
// ---------------------------------------------------------------------------

/// hwpush 负载 msgContent 的字段（serde camelCase 序列化后）
pub const HW_MSG_FIELDS: &[&str] = &[
    "msgId",
    "scheduleTaskId",
    "scheduleTaskName",
    "summary",
    "result",
    "content",
    "source",
    "taskFinishTime",
];

/// hwpush 默认推送服务地址（与 config/profile.rs 一致）
pub const HW_DEFAULT_SERVICE_URL: &str =
    "https://hiboard-claw-drcn.ai.dbankcloud.cn/distribution/message/cloud/claw/msg/upload";

/// hwpush 内容长度上限（与 core/validator.rs 一致）
pub const HW_MAX_CONTENT_LENGTH: u64 = 5000;

/// skill 最新版 config.json 中的关键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfigFile {
    /// 推送服务 URL（注意 skill 中为驼峰命名）
    #[serde(default, rename = "pushServiceUrl")]
    pub push_service_url: String,
    /// 最大内容长度
    #[serde(default)]
    pub max_content_length: u64,
    /// 默认任务结果
    #[serde(default)]
    pub default_result: String,
    /// 超时时间（秒）
    #[serde(default)]
    pub timeout: u64,
}

/// 从 task_pusher.py 源码中提取 `required_fields = [...]` 列表（API 必填字段）
pub fn parse_required_fields(source: &str) -> Option<Vec<String>> {
    let rest = source.find("required_fields")?;
    let rest = &source[rest..];
    let open = rest.find('[')?;
    let close_rel = rest[open..].find(']')?;
    let inner = &rest[open + 1..open + close_rel];

    let mut fields = Vec::new();
    let mut cursor = inner;
    while let Some(q) = cursor.find(['\'', '"']) {
        let quote = cursor[q..].chars().next()?;
        let after = &cursor[q + 1..];
        let end = after.find(quote)?;
        fields.push(after[..end].to_string());
        cursor = &after[end + 1..];
    }
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// 单条差异记录
#[derive(Debug, Clone, Serialize)]
pub struct DiffItem {
    /// 对比项名称（service_url / fields / max_length）
    pub key: &'static str,
    /// 是否一致（None 表示 skill 侧无法解析）
    pub matched: Option<bool>,
    /// skill 最新版的值
    pub skill_value: String,
    /// hwpush 当前的值
    pub hwpush_value: String,
}

/// 最新版 skill 与 hwpush 实现的兼容性对比结果
#[derive(Debug, Clone, Serialize)]
pub struct Compatibility {
    pub service_url: DiffItem,
    pub fields: DiffItem,
    pub max_length: DiffItem,
}

impl Compatibility {
    /// 是否完全兼容（三项全部一致且均可验证）
    pub fn is_compatible(&self) -> bool {
        self.service_url.matched == Some(true)
            && self.fields.matched == Some(true)
            && self.max_length.matched == Some(true)
    }

    /// 存在差异的项（需要用户关注的）
    pub fn diffs(&self) -> Vec<&DiffItem> {
        vec![&self.service_url, &self.fields, &self.max_length]
            .into_iter()
            .filter(|d| d.matched != Some(true))
            .collect()
    }
}

/// 对比最新版 skill（config.json + task_pusher.py）与 hwpush 的实现
pub fn check_compatibility(config_json: &str, task_pusher_py: &str) -> Compatibility {
    let skill_cfg: Option<SkillConfigFile> = serde_json::from_str(config_json).ok();

    // 1. 推送服务地址
    let service_url = match &skill_cfg {
        Some(cfg) if !cfg.push_service_url.is_empty() => DiffItem {
            key: "service_url",
            matched: Some(cfg.push_service_url == HW_DEFAULT_SERVICE_URL),
            skill_value: cfg.push_service_url.clone(),
            hwpush_value: HW_DEFAULT_SERVICE_URL.into(),
        },
        _ => DiffItem {
            key: "service_url",
            matched: None,
            skill_value: "(config.json 解析失败)".into(),
            hwpush_value: HW_DEFAULT_SERVICE_URL.into(),
        },
    };

    // 2. msgContent 必填字段
    let fields = match parse_required_fields(task_pusher_py) {
        Some(mut skill_fields) => {
            let mut hw_fields: Vec<String> = HW_MSG_FIELDS.iter().map(|s| s.to_string()).collect();
            // 忽略顺序，按排序后逐项比较
            skill_fields.sort();
            hw_fields.sort();
            DiffItem {
                key: "fields",
                matched: Some(skill_fields == hw_fields),
                skill_value: skill_fields.join(", "),
                hwpush_value: hw_fields.join(", "),
            }
        }
        None => DiffItem {
            key: "fields",
            matched: None,
            skill_value: "(task_pusher.py 未找到 required_fields)".into(),
            hwpush_value: HW_MSG_FIELDS.join(", "),
        },
    };

    // 3. 内容长度上限
    let max_length = match &skill_cfg {
        Some(cfg) if cfg.max_content_length > 0 => DiffItem {
            key: "max_length",
            matched: Some(cfg.max_content_length == HW_MAX_CONTENT_LENGTH),
            skill_value: cfg.max_content_length.to_string(),
            hwpush_value: HW_MAX_CONTENT_LENGTH.to_string(),
        },
        _ => DiffItem {
            key: "max_length",
            matched: None,
            skill_value: "(config.json 解析失败)".into(),
            hwpush_value: HW_MAX_CONTENT_LENGTH.to_string(),
        },
    };

    Compatibility {
        service_url,
        fields,
        max_length,
    }
}

/// 拉取 skill 包内的单个文件内容（最新版）
pub fn fetch_file(registry: &str, path: &str, timeout_secs: u64) -> Result<String, String> {
    let base = registry.trim_end_matches('/');
    let url = format!("{base}/api/v1/skills/{SKILL_SLUG}/file?path={path}");

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("网络错误: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    resp.text().map_err(|e| format!("读取响应体失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.0.17"), Some(vec![1, 0, 17]));
        assert_eq!(parse_version("v1.0.17"), Some(vec![1, 0, 17]));
        assert_eq!(parse_version("2.0"), Some(vec![2, 0]));
        assert_eq!(parse_version("1"), Some(vec![1]));
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("abc"), None);
        assert_eq!(parse_version("1.0.x"), None);
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.0.17", "1.0.17"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.0.17", "1.0.9"), Some(Ordering::Greater));
        assert_eq!(compare_versions("1.0.9", "1.0.17"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0.17", "1.0"), Some(Ordering::Greater));
        assert_eq!(compare_versions("2.0", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(
            compare_versions("v1.0.17", "1.0.16"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_versions("1.0.17", "x"), None);
    }

    #[test]
    fn test_is_newer_than() {
        assert!(is_newer_than("1.0.18", "1.0.17"));
        assert!(!is_newer_than("1.0.17", "1.0.17"));
        assert!(!is_newer_than("1.0.16", "1.0.17"));
    }

    #[test]
    fn test_newer_versions() {
        let versions = vec![
            SkillVersion {
                version: "1.0.18".into(),
                changelog: "c3".into(),
                changelog_source: "user".into(),
            },
            SkillVersion {
                version: "1.0.17".into(),
                changelog: "c2".into(),
                changelog_source: "user".into(),
            },
            SkillVersion {
                version: "1.0.16".into(),
                changelog: "c1".into(),
                changelog_source: "user".into(),
            },
        ];
        let newer = newer_versions("1.0.17", &versions);
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].version, "1.0.18");
        assert!(newer_versions("9.9.9", &versions).is_empty());
    }

    #[test]
    fn test_parse_response() {
        let json = r#"{
            "items": [
                {"version": "1.0.17", "changelog": "修复 Markdown 格式问题", "changelogSource": "user"},
                {"version": "1.0.16", "changelog": "切换更安全的请求域名"}
            ]
        }"#;
        let resp: VersionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.items[0].version, "1.0.17");
        assert_eq!(resp.items[0].changelog, "修复 Markdown 格式问题");
        assert_eq!(resp.items[0].changelog_source, "user");
        // changelogSource 缺失时默认空字符串
        assert_eq!(resp.items[1].changelog_source, "");
    }

    #[test]
    fn test_parse_required_fields() {
        let py = r#"
def validate_push_data(self, push_data):
    if 'authCode' not in push_data:
        return False, "缺少authCode字段"
    msg_content = push_data['msgContent']
    required_fields = ['msgId', 'scheduleTaskId', 'scheduleTaskName', 'summary', 'result', 'content', 'source', 'taskFinishTime']
    for field in required_fields:
        if field not in msg_content[0]:
            return False, f"缺少字段: {field}"
"#;
        let fields = parse_required_fields(py).unwrap();
        assert_eq!(
            fields,
            vec![
                "msgId",
                "scheduleTaskId",
                "scheduleTaskName",
                "summary",
                "result",
                "content",
                "source",
                "taskFinishTime"
            ]
        );
        // 无 required_fields 时返回 None
        assert!(parse_required_fields("no such list here").is_none());
        // 空列表返回 None
        assert!(parse_required_fields("required_fields = []").is_none());
    }

    #[test]
    fn test_check_compatibility_match() {
        let config_json = r#"{
            "pushServiceUrl": "https://hiboard-claw-drcn.ai.dbankcloud.cn/distribution/message/cloud/claw/msg/upload",
            "max_content_length": 5000,
            "timeout": 30,
            "default_result": "任务已完成"
        }"#;
        let py = "required_fields = ['taskFinishTime', 'msgId', 'content', 'scheduleTaskName', 'summary', 'source', 'result', 'scheduleTaskId']";
        let compat = check_compatibility(config_json, py);
        assert!(compat.is_compatible());
        assert!(compat.diffs().is_empty());
    }

    #[test]
    fn test_check_compatibility_diff() {
        // 字段少一个 + URL 变化 + 长度变化 → 全部不匹配
        let config_json = r#"{
            "pushServiceUrl": "https://new-domain.example.com/upload",
            "max_content_length": 10000,
            "timeout": 30
        }"#;
        let py = "required_fields = ['msgId', 'scheduleTaskId', 'summary', 'result', 'content', 'source']";
        let compat = check_compatibility(config_json, py);
        assert!(!compat.is_compatible());
        let diffs = compat.diffs();
        assert_eq!(diffs.len(), 3);
        let keys: Vec<_> = diffs.iter().map(|d| d.key).collect();
        assert!(keys.contains(&"service_url"));
        assert!(keys.contains(&"fields"));
        assert!(keys.contains(&"max_length"));
        // skill 侧字段缺失时 matched 为 Some(false)
        assert_eq!(compat.fields.matched, Some(false));
    }

    #[test]
    fn test_check_compatibility_unparseable() {
        // config.json 解析失败、task_pusher.py 无 required_fields → 无法验证
        let compat = check_compatibility("not json", "no required fields here");
        assert!(!compat.is_compatible());
        assert_eq!(compat.service_url.matched, None);
        assert_eq!(compat.fields.matched, None);
        assert_eq!(compat.max_length.matched, None);
    }
}
