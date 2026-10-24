#!/usr/bin/env bash
# Windows-only QA guard. Never executes an app, build, or test on the VPS.
# Usage: when-away.sh --idle | --dry-run [minimum >= 600] | --wait | --shell 'PowerShell' | --stop
set -euo pipefail
G=$(cd "$(dirname "$0")" && pwd)
S=/tmp/claude-1000/-home-valentin-projects-hypertask/dda64bba-e2ec-4ae7-bd77-524806065ad6/scratchpad
W='C:\Users\valen\AppData\Local\Temp\htbehavior'
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
  --wait|--shell)
    while :; do
      result=$(lh shell "& '$W\laptop-idle.ps1' -Mode Wait" --timeout 110)
      printf '%s\n' "$result"
      if [[ "$result" == *AWAY_READY* ]]; then break; fi
    done
    [[ "$mode" == --shell ]] || exit 0
    command=${2:?PowerShell command required}
    encoded=$(printf '%s' "$command" | iconv -f UTF-8 -t UTF-16LE | base64 -w0)
    # The laptop-local watchdog survives transport loss; its lease expires after 115s.
    lh shell "\$ErrorActionPreference='Stop'; if (-not (Test-Path '$W\away-heartbeat.txt') -or ([DateTime]::UtcNow - (Get-Item '$W\away-heartbeat.txt').LastWriteTimeUtc).TotalSeconds -gt 5) { Remove-Item '$W\away-interrupted.txt','$W\away-heartbeat.txt' -ErrorAction SilentlyContinue; [IO.File]::WriteAllText('$W\away-lease.txt','away'); Start-Process powershell.exe -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','$W\laptop-idle.ps1','-Mode','Monitor' -RedirectStandardOutput '$W\away-monitor.txt' -RedirectStandardError '$W\away-monitor.err' }; [IO.File]::WriteAllText('$W\away-lease.txt','away'); for (\$i=0; \$i -lt 20 -and -not (Test-Path '$W\away-heartbeat.txt'); \$i++) { Start-Sleep -Milliseconds 100 }; & '$W\laptop-idle.ps1' -Mode Assert; if (Test-Path '$W\away-interrupted.txt') { throw 'Owner returned; step cancelled' }; powershell.exe -NoProfile -ExecutionPolicy Bypass -EncodedCommand '$encoded'; if (\$LASTEXITCODE -ne 0) { throw 'Guarded step failed' }" --timeout 110
    ;;
  *) echo 'Usage: when-away.sh --idle|--dry-run [minimum >=600]|--wait|--shell command|--stop' >&2; exit 2 ;;
esac
