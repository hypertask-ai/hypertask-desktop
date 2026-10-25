# Visual parity round 8

## Acceptance, recorded before implementation

- Preserve the frozen live reference, fixture content, comparer, tolerances and existing assertions.
- Download SHA 72aeb6fb's layout-dumps. Report all original comparison counts, not filtered counts.
- Audit unsampled rail controls against web source. Add separate source-backed coverage, never relabel collector gaps as product improvements.
- Prove any presentation fix with real hosted native output and rejection of its pre-fix state.
- Require six successful exact-head CI jobs and a valid artifact receipt. No VPS execution or laptop access.

## Baseline

Web source revision `6f9901764d355aba7bfc76e4e62ff255e0b68b31` at `/home/valentin/projects/hypertask/src`.
Desktop `72aeb6fbef22e125472c1e3427955a948c6ace00`, [CI 40150377661](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40150377661).
Downloaded artifact `12582144767`, digest `sha256:89069a374784221d8546cd72e05793cfd5dd4729ce0ea6cb3f610cc4b0fc70bc`.

| Original comparison | Before |
| --- | ---: |
| Dense | 68 |
| Sparse | 34 |
| Reference content | 24 |
| Reference shared boxes | 0 |

All 24 reference differences are `rail.icon.5` through `rail.icon.12`, eight per screen. They say `only in app` because the frozen collector samples only icons 0 through 4. Removing real controls or modifying the frozen reference would manufacture a reduction. Inbox, board and task already have zero shared-box differences in that artifact.

## Source audit

Paths below are relative to `/home/valentin/projects/hypertask/src`.

| Property | Authority |
| --- | --- |
| Rail item 36 by 34px, 5px radius, hover background | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:65-87` |
| Expanded labels stay muted, including selected and hovered items | `AppShellRail.tsx:83-85` |
| Rail height, 8px vertical padding, 48px collapsed width | `AppShellRail.tsx:151-154` |
| Calendar, AI and search follow Table with 4px gaps | `AppShellRail.tsx:191,226-285` |
| Bottom group auto margin, 4px gaps and 4px bottom padding | `AppShellRail.tsx:303-360` |
| Button hover foreground becomes white/black | `AppShellRail.tsx:243,256,276,308,321,333,347,356` |
| Graphite hover background, foreground, muted foreground | `styles/tailwindThemes/graphite.css:14,53-55` |

The original idle dumps do not exercise hover. Native rail buttons currently show tooltips but retain muted icons and no hover background. Expanded selected labels also inherit the active icon color instead of the source's independent muted label color. These are source-backed presentation defects, not a reason to change sampled idle coordinates.

Proof uses the existing measured layout records and painted quads after simulated pointer movement. The native hover regression is added before its fix, so hosted CI can demonstrate the failure. A separate idle rail audit covers all 24 previously unsampled records without changing the original comparison or counting coverage as a visual improvement.
