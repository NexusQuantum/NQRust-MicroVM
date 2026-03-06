# License Settings Tab — Design Document

**Date:** 2026-02-23
**Status:** Approved

## Problem

Users have no in-app way to:
- View their full license details after initial activation
- Re-read the EULA
- Update/replace their license key without navigating to `/setup/license` (a setup-flow page, not a settings page)

## Solution

Add a dedicated **License** tab (6th tab) to `apps/ui/app/(dashboard)/settings/page.tsx`.

## Approach

**B — Separate component file.** Extract the full tab content to `apps/ui/components/license/license-settings-tab.tsx` and import it in `settings/page.tsx`. This keeps the already-large settings file manageable.

## Component: `LicenseSettingsTab`

**File:** `apps/ui/components/license/license-settings-tab.tsx`

### Section 1 — License Status Card

Displays all license fields from `useLicenseStatus()`:

- Status badge (Active / Grace Period / Invalid)
- Product, Customer Name
- License Key (masked, monospace)
- Expires At
- Activations / Max Activations
- Grace period warning banner (if `is_grace_period`)

Below the status details, a collapsible "Update License Key" section:
- Collapsed by default when license is active
- Auto-expanded when `!is_licensed`
- Input: `XXXX-XXXX-XXXX-XXXX` auto-formatter (same as setup page)
- Button: "Activate" — calls `useActivateLicense()`, shows toast, invalidates `licenseStatus` query
- On success: collapses form, refreshes status card

### Section 2 — EULA Card

Displays the EULA document read-only (no acceptance checkbox/button needed in settings — EULA is already accepted to reach this page):

- Header: "End User License Agreement" + version badge
- Language selector: English / Bahasa Indonesia (same `/eula/EULA.md` and `/eula/EULA_id.md` static files)
- ScrollArea with `ReactMarkdown` renderer
- Fixed height (e.g. `h-[400px]`) so it doesn't stretch the page

### Layout

```
[Account] [Appearance] [Logging] [Defaults] [System] [License]

┌─ License Status ──────────────────────────────────────┐
│  ● Active    Product: ...  Customer: ...              │
│  Key: XXXX-****-****-XXXX   Expires: ...              │
│  Activations: 1 / 5                                   │
│                                                       │
│  [Update License Key ▼]                               │
│    [XXXX-XXXX-XXXX-XXXX]  [Activate]                  │
└───────────────────────────────────────────────────────┘

┌─ End User License Agreement ──────────────────────────┐
│  v1.0.0  [English ▼]                                  │
│  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄   │
│  (scrollable EULA content — read-only, h-[400px])     │
│                                                       │
└───────────────────────────────────────────────────────┘
```

## Changes to `settings/page.tsx`

1. Add `License` tab trigger after `System` tab
2. Add `TabsContent value="license"` rendering `<LicenseSettingsTab />`
3. Remove `LicenseInfoCard` component from the Account tab (lines ~78–156)
4. Replace with a one-line note: *"Manage your software license in the **License** tab."*
5. Remove now-unused `useLicenseStatus` import from settings/page.tsx (it moves to the new component)

## Hooks Used (no new hooks needed)

| Hook | Source |
|------|--------|
| `useLicenseStatus()` | `lib/queries.ts` |
| `useActivateLicense()` | `lib/queries.ts` |
| `useEulaInfo()` | `lib/queries.ts` |

## Files Changed

| File | Action |
|------|--------|
| `apps/ui/components/license/license-settings-tab.tsx` | **Create** |
| `apps/ui/app/(dashboard)/settings/page.tsx` | **Modify** — add tab, remove `LicenseInfoCard` |

No backend changes required.

## Out of Scope

- Offline `.lic` file upload (already disabled in setup page, skip here too)
- License deactivation UI
- Per-user EULA re-acceptance (EULA is app-level; read-only view only)
