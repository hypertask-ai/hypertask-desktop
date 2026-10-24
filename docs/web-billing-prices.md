# Public web billing-price evidence

Source inspected 2026-10-24 (no credentials or browser session):
<https://app.hypertask.ai/_next/static/chunks/21973-32c3602a71702ca7.js>, minified line 1, module `95282`.
SHA-256 of the fetched JavaScript: `c5d2021f080113ab043f96ae9f5a934fab061cafc083959bc94572a9ac4739f5`.

`src/lib/subscriptionPlans.ts:28-40` maps these eight public values into
`allMonthlyPrices` / `allYearlyPrices`; module `85887` implements
`src/lib/planFromStripePriceId.ts:26-65`, including canonical Pro precedence.

```js
95282:(e,t,r)=>{r.d(t,{Eb:()=>p,ld:()=>d,vt:()=>m});let o=String("price_1OSg0zIhmcH60VccTpAFDv32"),i=String("price_1OSg35IhmcH60VccJP5LQo03"),n=String("price_1QjJeDIhmcH60VcciHzZ3mTJ"),l=String("price_1QjJf8IhmcH60VccNUrF7YJf"),s=String("price_1TMlqCIhmcH60Vccgq4fpSWu"),a=String("price_1TMlqCIhmcH60VccI6VjW2FM"),c=String("price_1QjJeDIhmcH60VcciHzZ3mTJ"),u=String("price_1QjJf8IhmcH60VccNUrF7YJf"),d=[o,n,s,c],p=[i,l,a,u],
```

| Public environment slot | Monthly | Yearly | Web plan |
| --- | --- | --- | --- |
| `_01` | `price_1OSg0zIhmcH60VccTpAFDv32` | `price_1OSg35IhmcH60VccJP5LQo03` | AI |
| `_02` | `price_1QjJeDIhmcH60VcciHzZ3mTJ` | `price_1QjJf8IhmcH60VccNUrF7YJf` | Pro (canonical precedence) |
| `_BYOK` | `price_1TMlqCIhmcH60Vccgq4fpSWu` | `price_1TMlqCIhmcH60VccI6VjW2FM` | BYOK |
| `_PRO` | `price_1QjJeDIhmcH60VcciHzZ3mTJ` | `price_1QjJf8IhmcH60VccNUrF7YJf` | Pro |

The native build uses these verified production values when public GitHub
build variables are absent/empty; an explicitly configured deployment can
replace each slot. Unknown prices map to Free, exactly as the web does.
These are public product identifiers, not Stripe secrets or authentication.
The server still authorizes every send; billing metadata is not authorization.
