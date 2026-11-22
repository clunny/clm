---
status: proposed
resolves: [clm-verify-refuses-on-an-invalid-table-in-the-checkout-s-own-clm]
---

## Intent

`clm verify` takes `host` from the CLM root's settings without refusing on a `[verify]`, `[affected]` or `[env]` table in the checkout's own `clm.toml` that it never uses.
