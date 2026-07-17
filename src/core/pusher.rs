use chrono::Utc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::config::profile::Config;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PushPayload {
    pub data: PayloadInner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PayloadInner {
    pub auth_code: String,
    pub msg_content: Vec<MsgContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MsgContent {
    pub msg_id: String,
    pub schedule_task_id: String,
    pub schedule_task_name: String,
    pub summary: String,
    pub result: String,
    pub content: String,
    pub source: String,
    pub task_finish_time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResponse {
    pub code: String,
    #[serde(default, alias = "desc")]
    pub message: String,
}

impl PushResponse {
    pub fn success(&self) -> bool {
        self.code == "0000000000"
    }
}

fn generate_msg_id() -> String {
    let ts = Utc::now().timestamp_millis();
    format!("hwpush_{ts}")
}

fn generate_trace_id() -> String {
    let ts = Utc::now().timestamp_millis();
    format!("hwpush_{ts:x}")
}

pub fn build_payload(
    auth_code: &str,
    name: &str,
    content: &str,
    result: &str,
    schedule_id: &Option<String>,
) -> PushPayload {
    let cfg = Config::default();
    let msg = MsgContent {
        msg_id: generate_msg_id(),
        schedule_task_id: schedule_id.clone().unwrap_or_default(),
        schedule_task_name: name.into(),
        summary: name.into(),
        result: result.into(),
        content: content.into(),
        source: cfg.defaults.source,
        task_finish_time: Utc::now().timestamp(),
    };

    PushPayload {
        data: PayloadInner {
            auth_code: auth_code.into(),
            msg_content: vec![msg],
        },
    }
}

pub fn push(url: &str, payload: &PushPayload, timeout_secs: u64) -> Result<PushResponse, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let trace_id = generate_trace_id();

    let resp = client
        .post(url)
        .header("x-trace-id", &trace_id)
        .json(payload)
        .send()
        .map_err(|e| format!("网络错误: {e}"))?;

    let status = resp.status();
    let body = resp
        .text()
        .map_err(|e| format!("读取响应体失败: {e}"))?;

    if !status.is_success() {
        return Err(format!("HTTP {status}: {body}"));
    }

    let push_resp: PushResponse = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {e}（响应体: {body}）"))?;

    Ok(push_resp)
}
