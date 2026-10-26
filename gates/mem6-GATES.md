# mem6 acceptance (before implementation)

- All builds, tests, lint, launches and image comparisons run on public GitHub CI only. No laptop access or VPS test copies.
- Inspect round-5 resident maps and owner receipts before selecting an allocation change. Maps identify backing/residency, not heap allocation stacks.
- Compare otherwise identical same-source production/control release builds, three fresh processes per inbox/board/task on unchanged demo and heavy fixtures.
- Preserve all frozen budgets, 1920x1009 maximized client geometry, DPI 96, seed 7381, timing and real warm restore/maximize redraws.
- No eviction, working-set limits, forced collection, fixture reduction, rendering suppression, screenshot tolerance or measurement-specific memory policy.
- Require lower median working set and private committed bytes at both capture points on every screen. Aim for heavy screens near 80 decimal MB; no laptop after-result claimed.
- Require byte-identical RGB screenshots (cold/warm) and identical layout JSON/counts. Retain native Windows/Linux interactions, fonts and all existing layout contracts.
- Prove the changed allocation owner through an actual capacity/requested-byte receipt; preserve stored data and visible content.
- Fetch/rebase before each push; draft PR only. Exact final SHA must have every required CI job green and nonexpired artifacts through the supplied receipt script.
- Report before/after working-set medians and private committed bytes per screen, exact SHA, PR/CI URLs and every job result.

## Baseline inspected

Round-5 CI 40276083867, source 82b22c4a3b7f3311d74af07f8812d4380a7f4785, downloaded read-only to /tmp/mem6-before.
Demo working set/private medians: inbox 56,500,224/30,597,120; board 61,706,240/37,404,672; task 73,265,152/48,103,424 bytes.
Heavy: inbox 69,238,784/36,794,368; board 87,764,992/56,270,848; task 89,915,392/58,945,536 bytes.
Heavy warm third-run private-region residency: 22,933,504/42,622,976/44,863,488 bytes. Image residency ~30.5 MB; mapped ~14–16.7 MB. Not additive to private commit.
Requested Rust heap: 12.27/26.56/32.55 MB; SQLite 266,184 bytes, fonts 1,042,912 bytes, fixed ClearType atlas 4,194,304 bytes. Mono/image caches are small; no avatar/description image decoder exists. Two swapchain buffers already minimum. Prior ClearType repacking changed pixels; prior mimalloc regressed.

## Work sequence

1. Trace retained UI/layout allocations against known owner sizes; choose one focused source-level change.
2. Add same-source CI control and exact owner/pixel/median comparison.
3. Inspect actual CI proof, fix causes without relaxing contracts, document accepted/rejected hypotheses.

Review is serial self-review; unavailable Cursor-only review tools are not claimed. Prove-it-works selects artifact-based proof; sequence-verifiable-units keeps allocation experiments separate.

## First focused allocation unit

Data shape: each InboxItem retains seven metadata fields after extracting typed identity/source. The SQL object starts with fifteen fields; IndexMap-backed serde_json retain drops values but keeps its larger backing allocation. Rebuild that one map with Map::with_capacity(retained_len), moving keys/values without cloning. Do not alter nested payloads, serialized content or order. The CI-only control omits packed-inbox-metadata; all other production features remain identical.

Rejected alternatives: another allocator (prior measured regression); font-atlas repacking (prior changed pixel); globally compacting arbitrary JSON trees (unnecessary breadth). This unit measures its own effect before pursuing larger UI allocations. Fixed-size atomic log2 buckets now report requested live Rust allocations by size on both builds, not stack attribution or an allocation change. No forced heap collection.

## First receipt and next unit

CI 40279953684 passed all eight build/control/native jobs and cold RGB/layout equality.
Packing lowered heavy requested Rust heap by 1,536,000 bytes on each screen and
cold WS 69.210 -> 67.486 / 87.667 -> 86.073 / 89.985 -> 88.535 MB.
The strict comparison rejected demo warm private commit: inbox 30,613,504 ->
31,281,152 bytes, board 38,023,168 -> 38,674,432, task 48,443,392 -> 48,906,240.
No gate or budget is relaxed. This unit alone is insufficient.

The size buckets show 6.95 MB of heavy-board requested live heap in 1–2 MiB
allocations, 5.02 MB of task heap in 64–128 KiB allocations, and multiple MB of
small text/UI allocations. These are requested allocation sizes, not stack traces.
Source identifies a concrete layout owner: ShapedLine embeds 32 DecorationRuns
for every line; WrappedLine reserves 32 even for one uniformly colored run.
Task readers retain vectors of these shaped lines for hit testing. Normal labels
also retain wrapped-line vectors. Only one style is common, not 32.

Selected shape: SmallVec with one inline decoration, and Vec starting at one.
Both grow normally for arbitrary style counts. No algorithm or public method changes.
A CI-only legacy-line-storage feature restores the old capacities. A size receipt
and size regression prove the owner changed; exact cold/warm RGB and full layout
comparison prove the original glyph/decorations/geometry remain. The pinned GPUI
crate is vendored with its unchanged license because these fields are private;
unsafe casts or mutating shared Cargo sources were rejected. The large source
import is mechanical; only two upstream source files change.

CI 40283074283 compiled the vendored crate on ARM64 and all platforms. The new
regression incorrectly equated font-run count with decoration count; the headless
text system returns one font run for the same font. Replaced that internal-shape
assertion with real painting of 64 alternating underline styles: require all 64
callback byte ranges and colors, plus wrapped-line painting and split lengths.
No production capacity or visual/memory acceptance is changed. The supplied
watcher hit the shared user's rate limit; direct read-only GitHub API logs retain
the actual failure. No test/build ran on this host.
