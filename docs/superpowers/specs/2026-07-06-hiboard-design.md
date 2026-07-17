# hboard 设计规格

## 概述

**hiboard** 是一个用 Rust 编写的全能任务管理 CLI 工具，核心功能是将任务结果推送到华为负一屏。它是 `today-task` OpenClaw Skill 的 Rust 原生替代品，不依赖 Python/OpenClaw 环境，提供更完整的任务管理工作流。

## 设计目标

- **轻量独立**：单一二进制，零运行时依赖
- **安全配置**：授权码通过 macOS Keychain 管理
- **完整工作流**：初始化 → 模板管理 → 推送 → 历史记录
- **兼容性**：与 `today-task` Skill 的负一屏 API 完全兼容

## 项目结构

```
playground/hiboard/
├── src/
│   ├── main.rs              # CLI 入口 + clap 定义
│   ├── cli/
│   │   ├── mod.rs           # 命令路由
│   │   ├── push.rs          # task-push 子命令
│   │   ├── init.rs          # init 子命令
│   │   └── template.rs      # template 子命令
│   ├── core/
│   │   ├── mod.rs
│   │   ├── pusher.rs        # 推送核心：格式化 + HTTP 请求
│   │   └── validator.rs     # 数据验证
│   ├── config/
│   │   ├── mod.rs
│   │   ├── profile.rs       # ~/.config/hiboard/config.toml 读写
│   │   └── keychain.rs      # macOS Keychain 集成
│   ├── template/
│   │   ├── mod.rs
│   │   └── manager.rs       # 模板管理：CRUD + 渲染（变量插值）
│   └── storage/
│       ├── mod.rs
│       └── history.rs       # SQLite 存储推送历史
├── templates/               # 内置模板
│   ├── daily.md
│   └── news.md
└── Cargo.toml
```

## CLI 命令设计

### `hiboard init` — 初始化

```bash
hiboard init
```

流程：
1. 检查 `~/.config/hiboard/config.toml`，不存在则创建默认配置
2. 引导输入授权码，存入 macOS Keychain
3. 可选配置推送 URL（回车使用默认 `https://hiboard-claw-drcn.ai.dbankcloud.cn/distribution/message/cloud/claw/msg/upload`）
4. 执行测试推送验证配置
5. 输出初始化结果

### `hiboard push` — 推送任务

```bash
# 从文件
hiboard push --file result.md --name "日报"

# 从 stdin
echo "# 日报" | hiboard push --name "日报"

# 使用模板
hiboard push --template daily --var project=hiboard

# 试运行
hiboard push --file report.md --name "测试" --dry-run
```

**参数：**

| 参数 | 必需 | 说明 |
|---|---|---|
| `--name` / `-n` | ✅ | 任务名称 |
| `--file` / `-f` | ❌ | Markdown 文件路径 |
| `--template` / `-t` | ❌ | 模板名称 |
| `--var` | ❌ | 模板变量 `key=value` |
| `--result` / `-r` | ❌ | 执行结果（默认"任务已完成"） |
| `--schedule-id` / `-s` | ❌ | 周期任务 ID |
| `--dry-run` | ❌ | 试运行 |

### `hiboard template` — 模板管理

```bash
hiboard template list           # 列出模板
hiboard template show daily     # 查看模板
hiboard template new my-report  # 创建模板
hiboard template edit my-report # 编辑模板（$EDITOR）
hiboard template delete my-report # 删除模板
```

### `hiboard config` — 配置管理

```bash
hiboard config get              # 查看配置
hiboard config set key value    # 修改配置
hiboard config auth             # 更新 Keychain 授权码
```

## 推送流程

```
用户输入 (文件/STDIN/模板)
        │
        ▼
  ┌─ formatter ──────────────────────┐
  │  1. 读取 Markdown 内容            │
  │  2. 从 Keychain 读取 authCode     │
  │  3. 生成 msgId / taskFinishTime    │
  │  4. 组装 PushPayload               │
  └──────────┬───────────────────────┘
             ▼
  ┌─ validator ───────────────────────┐
  │  1. 验证必需字段                   │
  │  2. 验证时间戳范围                 │
  │  3. 验证内容长度 ≤ 5000            │
  └──────────┬───────────────────────┘
             ▼
  ┌─ pusher ──────────────────────────┐
  │  1. 包装 { "data": payload }      │
  │  2. POST → 负一屏 API             │
  │  3. 解析响应 code                 │
  └──────────┬───────────────────────┘
             ▼
  ┌─ storage ─────────────────────────┐
  │  1. 记录到 SQLite                 │
  │  2. 输出结果到 stdout             │
  └───────────────────────────────────┘
```

## 核心数据结构

```rust
// 推送到负一屏的载荷
struct PushPayload {
    auth_code: String,
    msg_content: Vec<MsgContent>,
}

struct MsgContent {
    msg_id: String,
    schedule_task_id: String,
    schedule_task_name: String,
    summary: String,
    result: String,
    content: String,           // Markdown
    source: String,            // 固定 "OpenClaw"
    task_finish_time: i64,     // UTC 秒级时间戳
}

// 推送响应
struct PushResponse {
    success: bool,
    code: String,
    message: String,
    task_id: String,
    push_time: String,
}

// 错误类型
enum PushError {
    Network(String),
    Auth(String),      // 0000900034
    Service(String),   // 0200100004
    Validation(String),
    Keychain(String),
}
```

## 配置系统

### `~/.config/hiboard/config.toml`

```toml
[push]
service_url = "https://hiboard-claw-drcn.ai.dbankcloud.cn/distribution/message/cloud/claw/msg/upload"
timeout_secs = 30
retry_count = 3
dry_run = false

[defaults]
result = "任务已完成"
source = "OpenClaw"

[storage]
history_db_path = "~/.local/share/hiboard/history.db"
```

### Keychain 存储

| Key | Service | Account |
|---|---|---|
| 授权码 | `hiboard` | `auth_code` |

读取优先级：Keychain → 环境变量 `HIBOARD_AUTH_CODE` → 报错

## 模板系统

模板文件格式：TOML front matter + Markdown body

```markdown
---
name: daily
description: 日报模板
variables:
  - name: project
    description: 项目名称
    required: true
  - name: status
    description: 完成状态
    default: "进行中"
---

# {{project}} 日报

## 执行状态

{{status}}

---

*生成时间: {{date}} {{time}}*
```

模板目录优先级：`~/.config/hiboard/templates/` > 内置模板

## 依赖清单

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "blocking"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
security-framework = "3"
dirs = "6"
thiserror = "2"
```

## 错误处理策略

| 错误类型 | 处理方式 |
|---|---|
| `Auth` (0000900034) | 提示重新 `hiboard init` 更新授权码 |
| `Service` (0200100004) | 解析 CP 子错误码，给出具体操作步骤 |
| `Network` | 提示检查网络，支持 `--retry N` 自动重试 |
| `Validation` | 输出字段级错误信息 |

## 与 today-task Skill 的兼容性

- 请求体格式完全兼容（外层 `data` 包装 + 内部字段一致）
- 响应解析兼容（`code: "0000000000"` 视为成功）
- 错误码映射一致（复用 `0000900034` / `0200100004` / CP 子码）

## 非功能性需求

- **输出格式**：成功/失败信息清晰可读，使用颜色区分（通过 termcolor 或类似库）
- **退出码**：0 成功 / 1 失败
- **日志**：`--verbose` 输出详细日志到 stderr
- **隐私**：日志中授权码脱敏显示（`abc***`）
