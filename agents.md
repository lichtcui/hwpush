# 智能体上下文（Agents Context）

## 项目概述

**hiboard** — 将任务结果（Markdown）推送到华为负一屏的 Rust CLI 工具。零依赖二进制替代品，用于取代 `today-task` OpenClaw Skill。

## Rust 版本与约定

- **版本**: Edition 2024（Rust 1.96+）
- **风格**: 标准 `cargo fmt`，使用 `clap` derive API 编写 CLI 参数
- **错误处理**: 库代码使用 `thiserror`；CLI 层将错误转换为用户可读的消息
- **依赖**: 力求精简，优先使用 Rust 标准库

## 源码地图

```
src/main.rs           → CLI 入口，clap 解析器
src/cli/mod.rs        → 命令路由（Init/Push/Template/Config）
src/cli/init.rs       → hiboard init
src/cli/push.rs       → hiboard push（文件/标准输入/模板 → 校验 → 推送 → 记录）
src/cli/template.rs   → hiboard template（列出/查看/新建/编辑/删除）
src/config/mod.rs     → 配置模块根
src/config/profile.rs → TOML 配置读写（~/.config/hiboard/config.toml）
src/config/keychain.rs→ macOS Keychain 集成（security-framework）
src/core/mod.rs       → 核心模块根
src/core/pusher.rs    → 负载构建 + HTTP POST 到负一屏 API
src/core/validator.rs → 内容校验（长度、必填字段）
src/template/mod.rs   → 模板模块根
src/template/manager.rs→ 模板增删改查、变量插值、Front-matter 解析
src/storage/mod.rs    → 存储模块根
src/storage/history.rs→ SQLite 推送历史（rusqlite）
templates/            → 内置模板（daily.md, news.md）
```

## 设计规则

1. **认证码流程**: Keychain → `HIBOARD_AUTH_CODE` 环境变量 → 报错。禁止将认证码存入配置文件。
2. **API 兼容性**: 请求/响应格式必须与 `today-task` Skill 完全一致。成功返回 `code: "0000000000"`。
3. **模板解析**: 用户目录（`~/.config/hiboard/templates/`）优先于内置模板。
4. **内容校验**: 最大 5000 字符；name 和 content 为必填字段。
5. **隐私保护**: 认证码在日志中须脱敏显示（`abc***` 格式）。

## 配置路径

| 用途 | 路径 |
|------|------|
| 配置文件 | `~/.config/hiboard/config.toml` |
| 用户模板 | `~/.config/hiboard/templates/` |
| SQLite 数据库 | macOS: `~/Library/Application Support/hiboard/history.db`<br>Linux: `~/.local/share/hiboard/history.db` |
| Keychain 服务 | `hiboard` / account `auth_code` |

## CLI 命令树

```
hiboard
├── init                          # 首次初始化
├── push -n <名称> [-f 文件] [-t 模板] [--var k=v] [--dry-run]
├── template
│   ├── list                      # 列出模板
│   ├── show <名称>               # 查看模板
│   ├── new <名称>                # 新建模板
│   ├── edit <名称>               # 编辑模板
│   └── delete <名称>             # 删除模板
└── config
    ├── get                       # 查看配置
    ├── set <键> <值>             # 设置配置
    └── auth                      # 更新认证码
```

## 测试理念

- 使用 Rust 原生 `#[test]` 对校验器、模板渲染、负载构建进行单元测试。
- HTTP 调用使用 mock 进行推送测试（后续引入 `mockito` 或 `wiremock`）。
- Keychain 操作为 macOS 专属；使用 mock 测试或在 CI 上跳过。

## 参考：today-task Skill

`today-task` OpenClaw Skill 是 hiboard 的参考实现。

```bash
# 安装参考实现以供查阅
npx clawhub@latest install today-task --registry=https://mirror-cn.clawhub.com --dir /tmp/skill-ref
```

关键参考文件：
- `scripts/task_pusher.py` — 负载格式化逻辑（与 hiboard 格式一致）
- `scripts/hiboards_client.py` — HTTP 客户端，带 x-trace-id 头、错误码处理
- `scripts/task_push.py` — CLI 入口
- `simple_example.json` / `task_output_temp.json` — 示例负载

### 负一屏展示类型

华为 Claw 平台使用不同的**展示模板**渲染推送消息，所有类型都通过同一个 `source: "OpenClaw"` 字段控制。`msgContent` 中的字段决定展示外观：

| 展示模板 | 关键字段 | 渲染效果 |
|---------|---------|---------|
| **标准任务卡片** | `scheduleTaskName` + `summary` + `result` + `content` | 标题栏、状态标签、完整 Markdown 正文 |
| **周期任务卡片** | 同上 + `scheduleTaskId`（非空） | 与标准一致，但可分组/重复 |
| **仅摘要卡片** | `summary` + `result`（省略 `content`） | 紧凑卡片，无可展开正文 |

用户发起的任务 `source` 字段始终为 `"OpenClaw"`。其他 source 值对应不同的 Claw 平台集成（与本项目无关）。

### 错误码参考

| 错误码 | 含义 |
|--------|------|
| `0000000000` | 成功 |
| `0000900034` | 认证码无效/未授权 |
| `0200100004` | 服务动态推送错误（检查 CP 子码） |
| `0000500001` | 缺少必要请求头（如 x-trace-id） |

`0200100004` 的 CP 子码：
- `82600017` — 设备离线或未登录华为账号
- `82600013` — 服务动态推送开关关闭（需在负一屏设置中开启）
- `82600005` — 服务云端暂时不可用

## AI 智能体提示

> 以下提示为 AI 编码助手在操作本项目时的行为指引。

### 语言与文化

- 本项目是中文项目，所有输出、注释、文档使用中文。
- README、AGENTS.md、commit message、代码注释均使用中文。
- 英文术语（如 API、CLI、HTTP、SQLite）保持英文原样。
- 中英文之间加空格，使用全角中文标点。

### 测试结构

```
tests/
├── e2e/
│   ├── README.md             # E2E 测试说明
│   ├── run.sh                # E2E 测试脚本（需网络和认证码）
│   ├── standard_card.md      # 标准任务卡片测试数据
│   ├── periodic_card.md      # 周期任务卡片测试数据
│   └── summary_card.md       # 仅摘要卡片测试数据
└── integration_test.rs       # Rust 集成测试（无需网络，可 CI 执行）
```

- **集成测试**（`tests/integration_test.rs`）：验证负载构建、内容校验、模板渲染、Front-matter 剥离。无需网络和 Keychain，可在 CI 中执行。
- **E2E 测试**（`tests/e2e/run.sh`）：覆盖 CLI → 负载构建 → API 请求 → 响应处理的完整链路。需配置认证码和网络连接。
- **单元测试**: 放置在对应模块文件中（`#[cfg(test)] mod tests`），用于验证器、模板渲染、负载构建等纯逻辑。

### 常见操作

- **添加新子命令**: 创建 `src/cli/new_cmd.rs`，在 `cli::Command` 中添加枚举变体，在 `cli::dispatch()` 中挂载分发。
- **添加依赖**: 同时更新 `Cargo.toml` 和设计文档 `docs/superpowers/specs/` 中的"依赖"章节。
- **添加内置模板**: 在 `templates/` 目录下放置 `.md` 文件并包含 TOML front matter。
