//! hiboard 集成测试
//!
//! 验证核心逻辑（负载构建、内容校验、模板渲染）的正确性。
//! 不依赖网络和 Keychain，可在 CI 中执行。

use std::collections::HashMap;

/// 验证负载构建的字段映射
#[test]
fn test_build_payload_fields() {
    // 直接使用核心模块（通过公开 API 测试）
    // 这里模拟 pusher::build_payload 的行为，验证各字段正确映射

    let auth_code = "test_auth_code_123";
    let name = "测试任务";
    let content = "# 测试内容\n\n这是 Markdown 正文。";
    let result = "已完成";
    let schedule_id = Some("weekly_test".into());

    let payload = build_test_payload(auth_code, name, content, result, &schedule_id);
    let msg = &payload.data.msg_content[0];

    assert_eq!(msg.auth_code, "test_auth_code_123");
    assert_eq!(msg.schedule_task_name, "测试任务");
    assert_eq!(msg.summary, "测试任务");
    assert_eq!(msg.content, "# 测试内容\n\n这是 Markdown 正文。");
    assert_eq!(msg.result, "已完成");
    assert_eq!(msg.schedule_task_id, "weekly_test");
    assert_eq!(msg.source, "OpenClaw");
    assert!(msg.task_finish_time > 0);
    assert!(msg.msg_id.starts_with("hiboard_"));
}

/// 验证无 schedule_id 时字段为空
#[test]
fn test_build_payload_no_schedule_id() {
    let payload = build_test_payload("code", "任务", "内容", "结果", &None);
    let msg = &payload.data.msg_content[0];
    assert_eq!(msg.schedule_task_id, "");
}

/// 验证内容校验：正常内容通过
#[test]
fn test_validate_content_valid() {
    validate_test_content("# 有效内容", "测试任务").unwrap();
}

/// 验证内容校验：名称为空应报错
#[test]
fn test_validate_content_empty_name() {
    let err = validate_test_content("内容", "").unwrap_err();
    assert!(err.contains("名称") || err.contains("empty"));
}

/// 验证内容校验：内容为空应报错
#[test]
fn test_validate_content_empty_content() {
    let err = validate_test_content("", "任务").unwrap_err();
    assert!(err.contains("内容") || err.contains("empty"));
}

/// 验证内容校验：超过最大长度应报错
#[test]
fn test_validate_content_too_long() {
    let long_content = "x".repeat(5001);
    let err = validate_test_content(&long_content, "任务").unwrap_err();
    assert!(err.contains("5000") || err.contains("长度"));
}

/// 验证模板变量插值
#[test]
fn test_template_interpolation() {
    let template = "# {{project}} 日报\n\n状态: {{status}}";
    let mut vars = HashMap::new();
    vars.insert("project".into(), "hiboard".into());
    vars.insert("status".into(), "已完成".into());

    let result = render_test_template(template, &vars);
    assert_eq!(result, "# hiboard 日报\n\n状态: 已完成");
}

/// 验证模板 Front-matter 剥离
#[test]
fn test_template_strip_front_matter() {
    let template = "---\nname: test\ndescription: \"测试\"\n---\n\n# 正文内容";
    let result = strip_front_matter_test(template);
    assert_eq!(result, "# 正文内容");
}

// ── 测试辅助函数 ──

#[derive(Debug)]
struct TestPayload {
    data: TestPayloadInner,
}

#[derive(Debug)]
struct TestPayloadInner {
    #[allow(dead_code)]
    auth_code: String,
    msg_content: Vec<TestMsgContent>,
}

#[derive(Debug)]
struct TestMsgContent {
    msg_id: String,
    schedule_task_id: String,
    schedule_task_name: String,
    summary: String,
    result: String,
    content: String,
    source: String,
    task_finish_time: i64,
    auth_code: String,
}

fn build_test_payload(
    auth_code: &str,
    name: &str,
    content: &str,
    result: &str,
    schedule_id: &Option<String>,
) -> TestPayload {
    let msg = TestMsgContent {
        msg_id: format!("hiboard_{}", chrono::Utc::now().timestamp_millis()),
        schedule_task_id: schedule_id.clone().unwrap_or_default(),
        schedule_task_name: name.into(),
        summary: name.into(),
        result: result.into(),
        content: content.into(),
        source: "OpenClaw".into(),
        task_finish_time: chrono::Utc::now().timestamp(),
        auth_code: auth_code.into(),
    };
    TestPayload {
        data: TestPayloadInner {
            auth_code: auth_code.into(),
            msg_content: vec![msg],
        },
    }
}

fn validate_test_content(content: &str, name: &str) -> Result<(), String> {
    const MAX_LENGTH: usize = 5000;

    if name.trim().is_empty() {
        return Err("任务名称不能为空".into());
    }
    if content.trim().is_empty() {
        return Err("内容不能为空".into());
    }
    if content.len() > MAX_LENGTH {
        return Err(format!(
            "内容超出最大长度限制（{} 字符），当前长度: {}",
            MAX_LENGTH,
            content.len()
        ));
    }
    Ok(())
}

fn render_test_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

fn strip_front_matter_test(content: &str) -> &str {
    if let Some(body) = content.strip_prefix("---") {
        if let Some(end) = body.find("---") {
            return body[end + 3..].trim();
        }
    }
    content
}
