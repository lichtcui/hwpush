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

## 状态

{{status}}

## 摘要

{{content}}

---

*生成时间: {{date}} {{time}}*
