#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_ID="$(date +"%Y%m%d-%H%M%S")"
WORK_DIR="${SCRIPT_DIR}/.verify-agent-turn-cli/${RUN_ID}"
LOG_DIR="${WORK_DIR}/logs"
STORE_DIR="${WORK_DIR}/sessions"

mkdir -p "${LOG_DIR}" "${STORE_DIR}"

if [[ ! -t 0 ]]; then
  echo "请在交互式终端运行此脚本，以便输入 API Key。" >&2
  exit 1
fi

read -r -s -p "请输入 BIGMODEL API Key: " BIGMODEL_API_KEY
echo

if [[ -z "${BIGMODEL_API_KEY}" ]]; then
  echo "BIGMODEL API Key 不能为空。" >&2
  exit 1
fi

run_cli() {
  local label="$1"
  shift
  local log_file="${LOG_DIR}/${label}.log"

  echo
  echo "== ${label} =="
  (
    cd "${REPO_ROOT}"
    cargo run -p agent-turn-cli -- "$@"
  ) 2>&1 | tee "${log_file}"
}

echo "== step-1: cargo test -p agent-turn-cli =="
(
  cd "${REPO_ROOT}"
  cargo test -p agent-turn-cli
) 2>&1 | tee "${LOG_DIR}/step-1-tests.log"

run_cli \
  "step-2-turn-1-create-session" \
  --api-key "${BIGMODEL_API_KEY}" \
  --store-dir "${STORE_DIR}" \
  "记住：我的代号是 Orion。只回复“已记住”。"

SESSION_ID="$(
  find "${STORE_DIR}" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | head -n1
)"

if [[ -z "${SESSION_ID}" ]]; then
  echo "未找到会话目录，无法继续验证多轮对话。" >&2
  echo "请检查日志: ${LOG_DIR}/step-2-turn-1-create-session.log" >&2
  exit 1
fi

echo "session_id=${SESSION_ID}"

run_cli \
  "step-3-turn-2-check-memory" \
  --api-key "${BIGMODEL_API_KEY}" \
  --store-dir "${STORE_DIR}" \
  --session-id "${SESSION_ID}" \
  "我刚才的代号是什么？只回答代号本身。"

if ! grep -Eiq "\bOrion\b" "${LOG_DIR}/step-3-turn-2-check-memory.log"; then
  echo "第 2 轮验证失败：未在输出中检测到 Orion。" >&2
  exit 1
fi

run_cli \
  "step-4-turn-3-check-summary-format" \
  --api-key "${BIGMODEL_API_KEY}" \
  --store-dir "${STORE_DIR}" \
  --session-id "${SESSION_ID}" \
  "把前两轮对话各用一句话总结。请严格按两行输出：第一轮：...<换行>第二轮：..."

TURN3_LOG="${LOG_DIR}/step-4-turn-3-check-summary-format.log"

if ! grep -q "^\[summary\]$" "${TURN3_LOG}"; then
  echo "第 3 轮验证失败：未找到独立的 [summary] 标签行。" >&2
  exit 1
fi

SUMMARY_LINE_NUM="$(grep -n "^\[summary\]$" "${TURN3_LOG}" | head -n1 | cut -d: -f1)"
NEXT_LINE="$(sed -n "$((SUMMARY_LINE_NUM + 1))p" "${TURN3_LOG}")"

if [[ -z "${NEXT_LINE//[[:space:]]/}" || "${NEXT_LINE}" == \[done\]* ]]; then
  echo "第 3 轮验证失败：[summary] 后未检测到独立内容行（换行格式不正确）。" >&2
  exit 1
fi

if ! grep -q "^\[done\] stats:" "${TURN3_LOG}"; then
  echo "第 3 轮验证失败：未找到 [done] stats 行。" >&2
  exit 1
fi

echo
echo "验证通过。"
echo "日志目录: ${LOG_DIR}"
