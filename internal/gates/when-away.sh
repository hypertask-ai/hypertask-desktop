#!/usr/bin/env bash
# Windows-only QA guard. Never executes an app, build, or test on the VPS.
# Usage: when-away.sh --idle | --dry-run [minimum >= 600] | --wait | --shell 'PowerShell' | --stop
set -euo pipefail
G=$(cd "$(dirname "$0")" && pwd)
S=/tmp/claude-1000/-home-valentin-projects-hypertask/dda64bba-e2ec-4ae7-bd77-524806065ad6/scratchpad
W='C:\Users\valen\AppData\Local\Temp\htbehavior'
wait_away() {
  local result
  while :; do
    result=$(lh shell "& '$W\laptop-idle.ps1' -Mode Wait" --timeout 110) || return
    printf '%s\n' "$result"
    if [[ "$result" == *AWAY_READY* ]]; then return; fi
  done
}
mode=${1:---idle}
# Upload is read-only with respect to the owner's application and requires no GUI.
lh shell "New-Item -ItemType Directory '$W' -Force | Out-Null" >/dev/null
bash "$S/lh-put.sh" "$G/laptop-idle.ps1" "$W\laptop-idle.ps1" >&2
case "$mode" in
  --idle) lh shell "& '$W\laptop-idle.ps1' -Mode Idle" ;;
  --dry-run)
    minimum=${2:-600}
    [[ "$minimum" =~ ^[0-9]+$ ]] && (( minimum >= 600 && minimum <= 2147483647 )) || exit 2
    lh shell "& '$W\laptop-idle.ps1' -Mode DryRun -MinIdle $minimum"
    ;;
  --stop) lh shell "& '$W\laptop-idle.ps1' -Mode Stop" ;;
  --wait) wait_away ;;
  --shell)
    command=${2:?PowerShell command required}
    wait_away
    encoded=$(printf '%s' "$command" | iconv -f UTF-8 -t UTF-16LE | base64 -w0)
    # The laptop-local watchdog survives transport loss; its lease expires after 115s.
    status=0
    lh shell "\$ErrorActionPreference='Stop'; & '$W\laptop-idle.ps1' -Mode Arm; powershell.exe -NoProfile -ExecutionPolicy Bypass -EncodedCommand '$encoded'; if (\$LASTEXITCODE -ne 0) { throw 'Guarded step failed' }" --timeout 110 || status=$?
    interrupted=$(lh shell "if (Test-Path '$W\away-interrupted.txt') { 'AWAY_INTERRUPTED'; Get-Content '$W\away-interrupted.txt' }") || exit 1
    if [[ "$interrupted" == *AWAY_INTERRUPTED* ]]; then
      printf '%s\n' "$interrupted"
      wait_away
      echo 'AWAY_READY_AGAIN: interrupted step must be restarted from its isolated fixture' >&2
      exit 75
    fi
    exit "$status"
    ;;
  *) echo 'Usage: when-away.sh --idle|--dry-run [minimum >=600]|--wait|--shell command|--stop' >&2; exit 2 ;;
esac
