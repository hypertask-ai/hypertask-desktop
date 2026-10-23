#!/usr/bin/env bash
# usage: ram-round.sh <pushed commit sha> <out dir>
set -euo pipefail
sha=$1; out=$(realpath -m "$2")
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo 'Expected full commit SHA'; exit 1; }
S=/tmp/claude-1000/-home-valentin-projects-hypertask/dda64bba-e2ec-4ae7-bd77-524806065ad6/scratchpad
G=/home/valentin/projects/hypertask-desktop-wt-public/internal/gates
W='C:\Users\valen\AppData\Local\Temp\htram'
mkdir -p "$out"
run=$(env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh run list -R hypertask-ai/hypertask-desktop --workflow CI --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId')
[[ "$run" =~ ^[0-9]+$ ]] || { echo "CI_NOT_FOUND $sha"; exit 1; }
env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh run watch "$run" -R hypertask-ai/hypertask-desktop --exit-status > "$out/ci-watch.txt" 2>&1
env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh run view "$run" -R hypertask-ai/hypertask-desktop --json databaseId,status,conclusion,headSha,url --jq '"RUN \(.databaseId) \(.status) \(.conclusion) \(.headSha) \(.url)"' | tee "$out/ci.txt"
grep -q "completed success $sha" "$out/ci.txt" || { echo "CI_FAILED see $out/ci.txt"; exit 1; }
a=$(env -u GH_TOKEN -u GITHUB_TOKEN /usr/bin/gh auth token)
R=https://api.github.com/repos/hypertask-ai/hypertask-desktop
id=$(curl -fsS -H "Authorization: Bearer $a" "$R/actions/runs/$run/artifacts" | jq -er '.artifacts[]|select(.name=="hypertask-desktop-windows")|.id')
curl -fsSL -H "Authorization: Bearer $a" -o "$out/win.zip" "$R/actions/artifacts/$id/zip"
(cd "$out" && unzip -o -q win.zip)
exec 9>/tmp/htram-round.lock
flock 9
lh shell "New-Item -ItemType Directory -Force '$W' | Out-Null; Copy-Item 'C:\Users\valen\AppData\Local\Temp\htmeasure\parity.db' '$W\parity.db' -Force" > "$out/setup.txt"
bash "$S/lh-put.sh" "$out/ht-desktop.exe" "$W\ht-ram.exe" | tee "$out/upload.txt"
bash "$S/lh-put.sh" "$G/ram-measure.ps1" "$W\ram-measure.ps1" >> "$out/upload.txt"
want=$(sha256sum "$out/ht-desktop.exe" | cut -d' ' -f1 | tr a-f A-F)
got=$(lh shell "(Get-FileHash '$W\ht-ram.exe').Hash" | tr -d '\r' | awk 'NF { last=$0 } END { print last }')
[ "$want" = "$got" ] || { echo 'UPLOAD_HASH_MISMATCH'; exit 1; }
lh shell "Remove-Item '$W\ram.json' -EA 0; powershell -NoProfile -ExecutionPolicy Bypass -File '$W\ram-measure.ps1' 2>&1" --timeout 110 | tr -d '\r' | tee "$out/laptop.txt"
for file in ram.json inbox.json board.json task.json inbox.png board.png task.png; do
  lh shell "[Convert]::ToBase64String([IO.File]::ReadAllBytes('$W\$file'))" | tr -d '\r\n' | base64 -d > "$out/$file"
done
printf '%s\n' "$sha" > "$out/sha.txt"
python3 - "$out" "$want" <<'PY'
import json, pathlib, statistics, sys
out = pathlib.Path(sys.argv[1])
data = json.loads((out / 'ram.json').read_text(encoding='utf-8-sig'))
assert data['ExeSHA256'] == sys.argv[2]
assert (data['DisplayWidth'], data['DisplayHeight']) == (1920, 1080)
assert len(data['Samples']) == 9
for screen in ('inbox', 'board', 'task'):
    samples = [s for s in data['Samples'] if s['Screen'] == screen]
    assert sorted(s['Run'] for s in samples) == [1, 2, 3]
    assert all(s['Maximized'] and s['Dpi'] == 96 for s in samples)
    median = next(m for m in data['Medians'] if m['Screen'] == screen)
    for key in ('WorkingSet64', 'PrivateMemorySize64'):
        assert median[key] == statistics.median(s[key] for s in samples)
    print(f"{screen}: WS={median['WorkingSet64']/1e6:.3f} MB private={median['PrivateMemorySize64']/1e6:.3f} MB")
print('RAM_ROUND_COMPLETE')
PY
