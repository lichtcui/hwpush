# hiboard

> 将任务结果（Markdown）推送到华为负一屏 — Rust CLI 工具。

hiboard 是一个轻量级、零依赖的二进制工具，用于替代 `today-task` OpenClaw Skill。它通过简单的 CLI 工作流，将 Markdown 格式的任务结果推送到华为负一屏。

## 特性

- 🚀 **快速且独立** — 单一 Rust 二进制文件，无需 Python/OpenClaw
- 🔐 **安全认证** — 认证码存储在 macOS Keychain 中
- 📝 **模板系统** — 可复用的 Markdown 模板，支持变量插值
- 📋 **推送历史** — 基于 SQLite 的推送记录管理
- ✅ **试运行模式** — 发送前验证负载内容
- 🔄 **API 兼容** — 与 `today-task` 负一屏 API 完全兼容

## 安装

```bash
cargo install hiboard
```

或从源码构建：

```bash
git clone <仓库地址> && cd hiboard
cargo build --release
./target/release/hiboard --help
```

## 展示类型（卡片模板）

华为负一屏根据负载中填充的字段不同，以不同样式渲染推送消息。所有类型共享同一个 `source: "OpenClaw"`。

| 类型 | 使用的字段 | 展示效果 |
|------|-----------|---------|
| **标准任务卡片** | `name` + 内容 + result | 标题栏、状态标签、完整 Markdown 正文 |
| **周期任务卡片** | 同上 + `--schedule-id` | 与标准一致，但可分组重复展示 |
| **仅摘要卡片** | `name` + result（无内容） | 紧凑卡片，无可展开正文 |

#### 使用 `--dry-run` 测试每种卡片类型：

```bash
# 1) 标准任务卡片 — 完整正文含详情
echo "# 今日工作

## 完成事项
- 重构推送模块
- 添加单元测试
- 修复已知 Bug

## 明日计划
- 性能优化" | hiboard push --name "开发日报" --result "3项完成" --dry-run

# 2) 周期任务卡片 — 相同格式，添加 schedule-id 用于重复
echo "# 周报

本周完成：项目上线" | hiboard push --name "周报" --schedule-id "weekly_report" --dry-run

# 3) 仅摘要卡片 — 精简内容，紧凑展示
echo "完成" | hiboard push --name "日常任务" --result "已完成" --dry-run
```

实际推送（先执行 `hiboard config auth` 配置认证码）：

```bash
echo "# 测试通过" | hiboard push --name "标准任务卡测试" --result "测试通过"
```

## 快速开始

```bash
# 初始化配置
hiboard init

# 从文件推送
hiboard push --file result.md --name "日报"

# 从标准输入推送
echo "# 快速笔记" | hiboard push --name "笔记"

# 使用模板
hiboard push --template daily --var project=hiboard

# 试运行以检查负载内容
hiboard push --file report.md --name "测试" --dry-run
```

## 命令

| 命令 | 说明 |
|------|------|
| `init` | 创建配置、将认证码存入 Keychain、准备目录 |
| `push` | 将任务内容推送到负一屏 |
| `template` | 列出、查看、创建、编辑、删除模板 |
| `config` | 查看/设置配置或更新 Keychain 认证码 |

### `push` 选项

| 参数 | 必填 | 说明 |
|------|------|------|
| `--name / -n` | ✅ | 任务名称 |
| `--file / -f` | ❌ | Markdown 文件路径 |
| `--template / -t` | ❌ | 模板名称 |
| `--var` | ❌ | 模板变量（`key=value`） |
| `--result / -r` | ❌ | 结果文本（默认："任务已完成"） |
| `--schedule-id / -s` | ❌ | 周期任务 ID |
| `--dry-run` | ❌ | 仅校验，不发送 |

## 配置

配置文件路径：`~/.config/hiboard/config.toml`

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
# macOS: ~/Library/Application Support/hiboard/history.db
# Linux: ~/.local/share/hiboard/history.db
history_db_path = "~/.local/share/hiboard/history.db"
```

认证码优先级：
1. macOS Keychain（通过 `hiboard config auth` 或 `hiboard init` 设置）
2. 环境变量 `HIBOARD_AUTH_CODE`

## 模板

模板使用带 TOML Front-matter 的 Markdown 文件，支持 `{{variable}}` 变量插值。

```markdown
---
name: daily
description: 日报模板
variables:
  - name: project
    description: 项目名称
    required: true
---

# {{project}} 日报

{{content}}
```

模板解析优先级：`~/.config/hiboard/templates/` > 内置模板。

## 项目结构

```
hiboard/
├── src/
│   ├── main.rs              # CLI 入口
│   ├── cli/                 # 命令路由（init, push, template, config）
│   ├── core/                # 推送引擎与校验
│   ├── config/              # TOML 配置与 Keychain
│   ├── template/            # 模板增删改查与渲染
│   └── storage/             # SQLite 推送历史
├── templates/               # 内置模板
│   ├── daily.md
│   └── news.md
├── tests/                   # 集成测试与 E2E 测试
│   ├── integration_test.rs  # 负载构建、校验、模板渲染（CI 可执行）
│   └── e2e/                 # E2E 测试数据与脚本（需认证码+网络）
└── Cargo.toml
```

## 故障排查：设备未收到通知

`code: "0000000000"` 表示 API **已接受**请求，但能否送达设备取决于以下条件：

1. **华为账号** — 设备必须登录了华为账号
2. **通知开关已开启** — 打开负一屏 → 点击头像 → 我的 → 动态管理 → 找到"AI 任务完成通知" → 确保场景开关和服务提供方开关均已开启
   - 若关闭，API 返回错误 `0200100004`，CP 子码 `82600013`
3. **设备在线** — 手机须有网络连接
   - 若离线，API 返回错误 `0200100004`，CP 子码 `82600017`
4. **认证码有效** — 认证码会定期过期；若遇到错误 `0000900034`，通过 `hiboard config auth` 更新

## 测试

```bash
# 运行集成测试（无需网络，可在 CI 执行）
cargo test

# 运行 E2E 测试（需配置认证码和网络连接）
./tests/e2e/run.sh
```

| 测试层级 | 位置 | 依赖 | 内容 |
|---------|------|------|------|
| **集成测试** | `tests/integration_test.rs` | 无 | 负载构建、内容校验、模板渲染、Front-matter 剥离（8 用例） |
| **E2E 测试** | `tests/e2e/` | 认证码 + 网络 | CLI 完整链路：三种负一屏卡片类型推送 |

## 错误码

| 错误码 | 含义 | 处理方式 |
|--------|------|---------|
| `0000900034` | 认证错误 | 运行 `hiboard config auth` 更新认证码 |
| `0200100004` | 服务错误 | 检查响应中的 CP 子码详情 |
| 网络错误 | 连接失败 | 检查网络连接，或调整配置中的 `push.retry_count` |
| 校验错误 | 负载无效 | 检查字段级别的错误信息 |

## 许可协议

MIT
