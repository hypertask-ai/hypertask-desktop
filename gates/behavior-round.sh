#!/usr/bin/env bash
# usage: behavior-round.sh <pushed commit sha> <out dir>
# Never executes the app/build/tests on the VPS. Only CI and htbehavior on the laptop.
set -euo pipefail
sha=${1:?commit SHA required}; out=$(realpath -m "${2:?output directory required}")
S=/tmp/claude-1000/-home-valentin-projects-hypertask/dda64bba-e2ec-4ae7-bd77-524806065ad6/scratchpad
G=$(cd "$(dirname "$0")" && pwd)
W='C:\Users\valen\AppData\Local\Temp\htbehavior'
mkdir -p "$out"
# Same SHA-bound CI/artifact gate as parity-round, but gh watch blocks instead of a sleep loop.
run=$(env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh api "repos/hypertask-ai/hypertask-desktop/actions/runs?head_sha=$sha&per_page=30" --jq '[.workflow_runs[] | select(.name=="CI")][0].id')
[[ "$run" =~ ^[0-9]+$ ]] || { echo "CI_NOT_FOUND $sha"; exit 1; }
env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh run watch "$run" -R hypertask-ai/hypertask-desktop --exit-status > "$out/ci-watch.txt" 2>&1
env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh api "repos/hypertask-ai/hypertask-desktop/actions/runs/$run" --jq '"RUN \(.id) \(.status) \(.conclusion) \(.head_sha) \(.html_url)"' | tee "$out/ci.txt"
grep -q "completed success $sha" "$out/ci.txt" || { echo "CI_FAILED see $out/ci.txt"; exit 1; }
a=$(env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh auth token)
R=https://api.github.com/repos/hypertask-ai/hypertask-desktop
id=$(curl --fail -s -H "Authorization: Bearer $a" "$R/actions/runs/$run/artifacts" | jq -er '.artifacts[]|select(.name=="hypertask-desktop-windows")|.id')
curl --fail -sL -H "Authorization: Bearer $a" -o "$out/win.zip" "$R/actions/artifacts/$id/zip"
(cd "$out" && unzip -o -q win.zip)
lh shell "New-Item -ItemType Directory '$W' -Force | Out-Null; Copy-Item 'C:\Users\valen\AppData\Local\Temp\htmeasure\parity.db' '$W\parity.db' -Force" > "$out/setup.txt"
bash "$S/lh-put.sh" "$out/ht-desktop.exe" "$W\ht-behavior.exe" | tee "$out/upload.txt"
want=$(sha256sum "$out/ht-desktop.exe" | cut -d' ' -f1 | tr a-f A-F)
got=$(lh shell "(Get-FileHash '$W\ht-behavior.exe').Hash" | tr -d '\r' | awk 'NF { last=$0 } END { print last }')
[ "$want" = "$got" ] || { echo "UPLOAD_HASH_MISMATCH"; exit 1; }
for file in behavior-input.ps1 behavior-cases.json; do
  lh write "$W\$file" "$(cat "$G/$file")" > /dev/null
done
lh shell "Remove-Item '$W\*.json','$W\*.db-wal','$W\*.db-shm' -EA 0;" > /dev/null
# Restore cases after clearing old evidence, preventing a stale PASS from a previous binary.
lh write "$W\behavior-cases.json" "$(cat "$G/behavior-cases.json")" > /dev/null
lh shell "powershell -NoProfile -ExecutionPolicy Bypass -File '$W\behavior-input.ps1' -CaseFile '$W\behavior-cases.json'" --timeout 110 | tr -d '\r' | tee "$out/input.txt"
lh shell "Compress-Archive -Path '$W\*.json','$W\*.db','$W\*.stdout','$W\*.stderr' -DestinationPath '$W\evidence.zip' -Force" > /dev/null
lh shell "[Convert]::ToBase64String([IO.File]::ReadAllBytes('$W\evidence.zip'))" | tr -d '\r\n' | base64 -d > "$out/evidence.zip"
(cd "$out" && unzip -o -q evidence.zip)
python3 "$G/behavior-verify.py" "$G/behavior-cases.json" "$out" | tee "$out/results.txt"
