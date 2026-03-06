# API Documentation UI — Design Document

**Date:** 2026-03-02
**Status:** Approved
**Approach:** Custom-built Stripe-style API docs in Next.js

## Context

Replace reliance on Swagger UI (`/docs` on the manager) with a polished, branded API documentation experience built into the Next.js frontend (`apps/ui`). Target audience: external developers integrating with the NQRust-MicroVM API.

## Architecture

### Route Structure

Docs live outside the `(dashboard)` layout group with their own layout optimized for reading:

```
apps/ui/app/
  docs/
    layout.tsx          — docs layout (sidebar + two-column)
    page.tsx            — landing page ("Getting Started")
    [[...slug]]/
      page.tsx          — dynamic route for any endpoint/guide
```

### Data Flow

```
openapi.json (from manager at build time)
  → scripts/parse-openapi.ts
  → apps/ui/content/api/*.json (structured per-tag)
  + apps/ui/content/api/overrides/*.mdx (hand-written enrichments)
  → Next.js pages render combined data
```

The parser runs at build time / dev server start, transforming the OpenAPI spec into per-tag JSON files. MDX overrides are merged at render time.

## Layout — Stripe-Style Two-Column

```
┌──────────────────────────────────────────────────────────┐
│  Logo    API Reference    Guides    Changelog   [Search] │
├────────┬─────────────────────────┬───────────────────────┤
│  Nav   │  LEFT (60%)             │  RIGHT (40%)          │
│        │                         │                       │
│  Auth  │  Endpoint title         │  Code Example         │
│  VMs   │  POST /v1/vms           │  cURL | JS | Python   │
│  ├ Crt │                         │                       │
│  ├ Lst │  Description            │  Response Example     │
│  ├ Get │                         │  200 | 400 | 401      │
│  ...   │  Parameters table       │                       │
│        │  (name, type, required) │  Playground           │
│  Imgs  │                         │  [Try it]             │
│  ...   │  Response Schema        │  Bearer: [____]       │
│        │  (expandable nested)    │  Body: { Monaco }     │
│        │                         │  [Send Request]       │
└────────┴─────────────────────────┴───────────────────────┘
```

- Sidebar: grouped by API tag, collapsible, with search
- Left column: description, param tables, response schema
- Right column: sticky code examples, response samples, interactive playground
- Dark/light mode via existing `next-themes`

## Core Components

### 1. ApiEndpoint
Main building block rendering one endpoint. Shows method badge (color-coded: GET=green, POST=blue, PUT=amber, DELETE=red, PATCH=purple), monospace path with highlighted params, description (from OpenAPI or MDX override), auth badge.

### 2. ParamTable
Parameter table grouped by location (path, query, body). Each row: name (mono), type, required indicator, description. Nested objects expandable inline. Enum values as badges.

### 3. CodeExample
Tab bar: cURL | JavaScript | Python. Auto-generated from endpoint spec with realistic sample values. Syntax-highlighted with shiki. Copy button per example.

### 4. ResponseViewer
Tabs per status code. JSON with syntax highlighting. Toggle between schema view and example value.

### 5. ApiPlayground
Collapsible panel in right column. Auth: Bearer token input (persisted in localStorage) with "Login" shortcut. Auto-detected path/query param inputs. Monaco editor for request body (pre-filled with example). Send button, response display with status/headers/body. Loading state with Spinner.

### 6. DocsSidebar
Search input (fuzzy, using cmdk). Sections per API tag, collapsible. Method badge + path per endpoint. Active item highlighted. Sticky, independently scrollable.

### 7. Landing Page
"Getting Started" guide: auth flow, base URL, rate limits. Quick links per API section. First-call example (login + create VM).

## Interactive Playground

- Reuses existing `ApiClient` from `lib/api/http.ts` with configurable base URL
- Default base URL from `NEXT_PUBLIC_API_BASE_URL`
- Users can override base URL via settings dropdown
- Bearer token input with "Login" shortcut (calls `/v1/auth/login`)
- Request history in localStorage (last 20 per endpoint)
- Same-origin CORS — works with existing setup

## MDX Override System

```
content/api/overrides/
  vms/
    create.mdx
    list.mdx
  auth/
    login.mdx
  _guides/
    authentication.mdx
    pagination.mdx
```

Each MDX file can export:
- `description` — replaces OpenAPI summary
- `examples` — custom code examples per language
- `notes` — callouts/warnings
- Default export — full custom content replacing auto-generated view

If no override exists, auto-generated content from OpenAPI is used.

## Tech Decisions

| Decision | Choice | Reasoning |
|----------|--------|-----------|
| Code highlighting | Monaco (playground editor), shiki (read-only blocks) | Monaco in deps; shiki lighter for static |
| OpenAPI parsing | Custom `scripts/parse-openapi.ts` | Simple transform, no heavy libs needed |
| MDX rendering | `next-mdx-remote` | Dynamic MDX without filesystem routing |
| Search | Client-side fuzzy via `cmdk` (already in deps) | Command palette style, no server needed |
| State | URL params for endpoint, localStorage for playground auth | Simple, shareable URLs |
| New deps | `shiki` + `next-mdx-remote` only | Minimal additions |

## API Surface

~80+ endpoints across 11 documented tags: Auth, Users, Hosts, Templates, VMs, VM Devices, Images, Snapshots, Functions, Containers, Logs. Additional undocumented features (licensing, networks, volumes, metrics) to be added to OpenAPI spec incrementally.
