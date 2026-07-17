#!/usr/bin/env bash
#
# hwpush E2E 测试脚本
# 测试三种负一屏卡片类型的推送
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_DIR/target/release/hwpush"

# 颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
PASS=0
FAIL=0

# 检查编译产物
if [ ! -f "$BINARY" ]; then
  echo -e "${RED}❌ 未找到编译产物，请先执行 cargo build --release${NC}"
  exit 1
fi

# 检查认证码
AUTH_CODE=""
if [ -n "${HWPUSH_AUTH_CODE:-}" ]; then
  AUTH_CODE="$HWPUSH_AUTH_CODE"
elif command -v security &>/dev/null; then
  AUTH_CODE=$(security find-generic-password -s hwpush -a auth_code -w 2>/dev/null || true)
fi

if [ -z "$AUTH_CODE" ]; then
  echo -e "${RED}❌ 未找到认证码。请设置 HWPUSH_AUTH_CODE 环境变量或通过 Keychain 配置。${NC}"
  exit 1
fi

export HWPUSH_AUTH_CODE="$AUTH_CODE"

run_test() {
  local name="$1"
  local file="$2"
  local result="$3"
  local extra="${4:-}"

  echo -e "\n${CYAN}━━━ 测试: $name ━━━${NC}"

  # shellcheck disable=SC2086
  if $BINARY push -n "$name" -f "$SCRIPT_DIR/$file" -r "$result" $extra 2>&1; then
    echo -e "${GREEN}✅ 通过: $name${NC}"
    PASS=$((PASS + 1))
  else
    echo -e "${RED}❌ 失败: $name${NC}"
    FAIL=$((FAIL + 1))
  fi
}

echo -e "${CYAN}══════════════════════════════════════${NC}"
echo -e "${CYAN}  hwpush E2E 测试套件${NC}"
echo -e "${CYAN}══════════════════════════════════════${NC}"

run_test "E2E-标准任务卡" "standard_card.md" "3项完成"
run_test "E2E-周期任务卡" "periodic_card.md" "全部完成" "-s e2e_weekly_001"
run_test "E2E-摘要卡" "summary_card.md" "已完成"

echo -e "\n${CYAN}══════════════════════════════════════${NC}"
echo -e "结果: ${GREEN}$PASS 通过${NC}, ${RED}$FAIL 失败${NC}"
echo -e "${CYAN}══════════════════════════════════════${NC}"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
