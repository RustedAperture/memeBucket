# Picker Add Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a shared multiline Add Links flow to the Tauri Picker and mobile `/picker`, with remembered bucket selection and one final success/failure summary.

**Architecture:** Keep all behavior in the shared React Picker page. Add a focused `PickerAddLinks` component for input, destination validation, sequential submissions, and summary state; the page will control whether Search or Add Links is visible and will persist the selected bucket. Reuse `POST /api/buckets/{bucket_id}/images` for every non-empty line, so the existing server-side image, X/Twitter, Bluesky, and Tenor processing remains unchanged.

**Tech Stack:** Next.js 16.2.7, React 19, TypeScript, Tailwind CSS, existing `apiGet`/`apiPost` helpers, Tauri-hosted webview, mobile `/picker` route.

## Global Constraints

- Use the shared `/picker` implementation for both Tauri and mobile; do not add a native-only Tauri add-media command.
- The first-ever Picker bucket selection is `all`; restore the last valid writable bucket from local storage on later openings.
- `all` is an aggregate/read-only view and cannot be submitted as an add destination; offer Inbox as a shortcut.
- Accept one link per line, ignore blank lines, trim each link, submit sequentially, clear the input after completion, and show only one aggregate summary.
- Update both `/Users/cameronvarley/projects/ezgif/changelog.md` and `/Users/cameronvarley/projects/ezgif/apps/web/public/changelog.json` for the feature.
- Preserve the existing Picker search, keyboard navigation, Tauri copy/paste, changelog banner, and focus refresh behavior.

---

### Task 1: Add pure link parsing and submission-result helpers

**Files:**
- Create: `apps/web/lib/picker-add-links.ts`

**Interfaces:**
- Produces `parsePickerLinks(value: string): string[]`, returning trimmed, non-empty lines in input order.
- Produces `isWritablePickerBucket(bucketId: string, buckets: Bucket[]): boolean`, treating `all` and subscribed/read-only buckets as invalid destinations.
- Produces `PickerAddLinksSummary`, `{ total: number; added: number; failed: number }`.

- [ ] **Step 1: Define the helpers and types**

```ts
import type { Bucket } from "@/lib/types";

export type PickerAddLinksSummary = {
  total: number;
  added: number;
  failed: number;
};

export function parsePickerLinks(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function isWritablePickerBucket(bucketId: string, buckets: Bucket[]): boolean {
  if (bucketId === "all") return false;
  const bucket = buckets.find((candidate) => candidate.id === bucketId);
  return Boolean(bucket && !bucket.is_subscribed && !bucket.is_read_only);
}
```

- [ ] **Step 2: Run the existing web checks before integration**

Run: `npm run build` from `apps/web`.

Expected: the existing production build passes before the new component is wired into the page.

- [ ] **Step 3: Commit the focused helper**

```bash
git add apps/web/lib/picker-add-links.ts
git commit -m "feat(web): add picker link helpers"
```

### Task 2: Build the shared Add Links panel

**Files:**
- Create: `apps/web/components/picker-add-links.tsx`
- Modify: `apps/web/lib/picker-add-links.ts` only if the component needs a helper export defined in Task 1

**Interfaces:**
- Consumes `Bucket[]`, the selected bucket ID, and callbacks for bucket selection and returning to search.
- Produces a self-contained Add Links panel with destination selection, multiline input, sequential API submissions, and a final summary.

- [ ] **Step 1: Define the component props**

```ts
type PickerAddLinksProps = {
  buckets: Bucket[];
  bucketId: string;
  onBucketChange: (bucketId: string) => void;
  onUseInbox: () => void;
  onBack: () => void;
};
```

- [ ] **Step 2: Add the destination and input UI**

Use the existing `Select`, `SelectContent`, `SelectGroup`, `SelectItem`, `SelectLabel`, and `SelectTrigger` components. Keep `All buckets` in the selector for consistency, show a warning when it is selected, and render a `Use Inbox` button that calls `onUseInbox`.

Render a `<textarea>` with one-link-per-line guidance and a submit button whose label changes between `Add links`, `Adding 1 of N…`, and `Done`.

- [ ] **Step 3: Implement sequential submission**

```ts
const links = parsePickerLinks(value);
let added = 0;
for (const url of links) {
  try {
    await apiPost(`/api/buckets/${bucketId}/images`, { url });
    added += 1;
  } catch {
    // Continue so the final summary includes both successes and failures.
  }
}
setSummary({ total: links.length, added, failed: links.length - added });
setValue("");
```

Guard submission when the parsed list is empty or `isWritablePickerBucket(bucketId, buckets)` is false. Keep the submit button disabled while the loop is running.

- [ ] **Step 4: Render the aggregate summary**

After the loop, show one summary with total, added, and failed counts. Do not render per-link result rows or retry controls. Keep a `Done` action that clears the summary and returns to the normal Add Links form without changing the selected bucket.

- [ ] **Step 5: Commit the panel**

```bash
git add apps/web/components/picker-add-links.tsx apps/web/lib/picker-add-links.ts
git commit -m "feat(web): add picker link submission panel"
```

### Task 3: Integrate mode switching and bucket persistence into Picker

**Files:**
- Modify: `apps/web/app/picker/page.tsx`

**Interfaces:**
- Consumes `PickerAddLinks` from Task 2.
- Produces the compact plus control beside the existing bucket dropdown and restores the last valid writable bucket.

- [ ] **Step 1: Add Picker mode and persistence state**

Add a `pickerMode` state with `"search" | "add-links"`, a storage key such as `picker.selectedBucketId`, and a first-mount effect that reads the stored bucket ID. Keep the initial React state as `all` to avoid browser-only access during render.

- [ ] **Step 2: Validate the restored bucket after buckets load**

After `fetchBuckets()` receives data, restore the stored ID only when `isWritablePickerBucket(storedId, loaded)` is true. Otherwise retain `all`. Persist every user-selected bucket after validating it against the loaded bucket list. Do not reset `bucketId` on focus, minimize, close, or reopen.

- [ ] **Step 3: Add the plus button beside the selector**

Wrap the existing bucket `Select` and a compact icon button in the current header row. Use a plus icon, `aria-label="Add media"`, and a tooltip/title. Clicking it sets `pickerMode` to `"add-links"`; it must not change `bucketId`.

- [ ] **Step 4: Render Add Links and Search modes**

When `pickerMode === "add-links"`, render `PickerAddLinks` in the main content area. Pass the current buckets and bucket ID, wire `onBucketChange` to the same persisted selection handler, wire `onUseInbox` to select the owned Inbox bucket when present, and wire `onBack` to return to search. When `pickerMode === "search"`, preserve the existing banner, grid, keyboard navigation, and image selection exactly as they work today.

If no owned Inbox exists, the Add Links panel must still require an explicit writable bucket and hide/disable the Inbox shortcut rather than selecting a subscribed or read-only bucket.

- [ ] **Step 5: Verify the shared desktop/mobile behavior**

Run the web app and manually verify:

1. `/picker` opens in Search mode with All selected on a fresh browser profile.
2. The plus beside the dropdown opens Add Links without changing the selected bucket.
3. Selecting a writable bucket persists it across reloads and returning to Search.
4. Closing/reopening the Tauri Picker preserves the selected bucket because the page is not remounted.
5. All selected blocks submission and offers Inbox when an owned Inbox exists.

- [ ] **Step 6: Commit the integration**

```bash
git add apps/web/app/picker/page.tsx
git commit -m "feat(web): integrate picker add links mode"
```

### Task 4: Add release notes and run final verification

**Files:**
- Modify: `changelog.md`
- Modify: `apps/web/public/changelog.json`

**Interfaces:**
- Consumes the completed shared Picker feature from Tasks 1–3.
- Produces matching user-facing `0.2.8` release notes in repository Markdown and web JSON.

- [ ] **Step 1: Add a concise user-facing Added entry**

Use this wording in both files:

`Added the ability to add multiple media links at once from the Picker using one link per line, with remembered bucket selection and a completion summary.`

Place it under the existing `0.2.8` `Added` section. Do not mention implementation details such as sequential requests, local storage, or build status.

- [ ] **Step 2: Validate the changelog JSON and diff**

Run from the repository root:

```bash
node -e "JSON.parse(require('fs').readFileSync('apps/web/public/changelog.json','utf8')); console.log('changelog.json valid')"
git diff --check
```

Expected: JSON validation prints `changelog.json valid` and `git diff --check` prints nothing.

- [ ] **Step 3: Run final verification**

Run:

```bash
cd apps/web && npm run build
cd ../.. && cargo test --manifest-path apps/server/Cargo.toml
```

Expected: the Next.js production build succeeds and the server test suite passes. Existing unrelated web lint failures are not part of this feature’s acceptance criteria.

- [ ] **Step 4: Commit the release notes**

```bash
git add changelog.md apps/web/public/changelog.json
git commit -m "docs: document picker add links"
```

## Plan self-review

- The plan covers the shared Tauri/mobile page, remembered bucket selection, All-bucket validation, Inbox shortcut, one-link-per-line parsing, sequential submission, aggregate summary, unchanged server API, changelog updates, and final verification.
- No native Tauri Rust changes are required because the desktop app already hosts the shared `/picker` route.
- No placeholder steps or unspecified APIs remain; the exact endpoint, props, storage key, helper signatures, commands, and release-note wording are defined above.
