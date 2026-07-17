# E2E 测试

> 端到端测试，覆盖 hiboard 从 CLI 输入 → 负载构建 → API 请求 → 响应处理的完整链路。

## 测试文件

| 文件 | 卡片类型 | 特点 |
|------|---------|------|
| `standard_card.md` | 标准任务卡片 | 完整 Markdown 正文，含标题、列表、粗体 |
| `periodic_card.md` | 周期任务卡片 | 带 `--schedule-id` 参数，用于分组/重复展示 |
| `summary_card.md` | 仅摘要卡片 | 极简内容，紧凑展示，无可展开正文 |

## 运行方式

```bash
# 方式一：手动指定文件
hiboard push -n "标准任务卡" -f tests/e2e/standard_card.md -r "3项完成"
hiboard push -n "周期任务卡" -f tests/e2e/periodic_card.md -r "全部完成" -s "weekly_001"
hiboard push -n "摘要卡" -f tests/e2e/summary_card.md -r "已完成"

# 方式二：运行自动化测试脚本
./tests/e2e/run.sh

# 方式三：使用标准输入
cat tests/e2e/standard_card.md | hiboard push -n "标准任务卡" -r "3项完成"
```

## 前置条件

- 已通过 `hiboard config auth` 或 `HIBOARD_AUTH_CODE` 环境变量配置认证码
- 网络连通，能访问华为负一屏 API
