# Command and keyboard gap evidence

Acceptance is `docs/gaps-acceptance.md`. No VPS build, test, lint, test copy or
app launch ran. No owner-laptop access occurred. Source/diff review is serial
self-review; no unavailable Cursor tool or independent reviewer is claimed.

## Actual negative controls

Test-only SHA `c25f4c2598b8a348b8c8ff9fd4d136692249dc0c` failed compilation
because the new assertion required Debug on Editing. The assertion was corrected
without changing production types or weakening its state check.

Corrected test-only SHA `c1516bf981e2b1715af5b82df272500f9a4de7cb`, CI
https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40242158385,
reports **203 passed, 2 failed** on all three native test platforms. Escape's
real input regression observes the old saved description `<p>Hidden</p>`, proving
that Escape had not queued the changed description. The other failure exposed
an incomplete new assignment fixture, not assignment behavior: apply_response
correctly rejected its missing required title. The fixture now supplies the
full task identity/title/status. This run is not a green receipt.

## Creation remains an exact-route blocker

`docs/creation-parity.md` records the web rules and source lines. N/Shift+C is
not silently mapped to AI compose or append-only creation. The exact POST
routes need the HTPR-7699 server sign-in work and the authoritative active-view
read model. Defaults are no longer described as unknown; no unexecutable
partial creation UI is introduced.

## Reference layout baseline

Baseline SHA `72aeb6fbef22e125472c1e3427955a948c6ace00`, successful CI
https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40150377661.
Downloaded `layout-dumps` artifact **12582144767**, digest
`sha256:89069a374784221d8546cd72e05793cfd5dd4729ce0ea6cb3f610cc4b0fc70bc`.
Its original `compare-reference.txt` reports **8 inbox / 8 board / 8 task = 24**.
Every record is `rail.icon.5` through `rail.icon.12`, only in app. The original
collector never sampled these eight real web controls. Shared reference boxes
and style fields have zero differences. Dense is 68; sparse is 34.

No comparer, tolerance, reference, fixture or native dump key is changed here.
Other visual edits belong to `look9`. Lowering 24 by deleting real controls or
filtering those records would not be a product improvement. Existing separate
source-backed rail and hover checks are retained. Final exact-head proof follows
in the draft PR receipt; intermediate results are not promoted to completion.
