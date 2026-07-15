# Picker Add Links Design

## Goal

Let users add media links from both the Tauri Picker and the mobile `/picker` page through one shared web experience.

## User experience

- The existing Picker keeps its current search and results view.
- A compact plus button appears beside the bucket dropdown.
- Selecting the plus button replaces the results area with an Add Links view; it does not open a second window or route.
- The Add Links view contains a multiline input with one link per line and an Add Links action.
- Blank lines are ignored. Links are trimmed before submission.
- The currently selected bucket is the destination.
- The first-ever Picker selection is All buckets. The last selected bucket is saved locally and restored whenever the Picker opens.
- All buckets is an aggregate/read-only view, not a valid write destination. When it is selected, the Add Links view requires a writable bucket and offers Use Inbox as a shortcut.
- After submission, the input is cleared and the user sees one summary containing successful and failed counts.
- Links are submitted sequentially through the existing image-add endpoint so the existing image, X/Twitter, Bluesky, and Tenor processing pipeline is reused.
- The same behavior is available in the desktop Tauri-hosted Picker and mobile `/picker`.

## Architecture

The implementation stays in the shared web Picker page. Tauri continues to host `/picker`, so no native Rust add-media flow is introduced. The page owns:

- Picker mode: search or add links.
- Selected bucket state and local persistence.
- Input parsing, submission progress, and final summary.
- The destination validation and Inbox shortcut when All buckets is selected.

The server API remains unchanged. Each non-empty line is sent to `POST /api/buckets/{bucket_id}/images` using the existing `{ url }` payload. Processing and storage behavior therefore remain centralized in the server’s current media pipeline.

## State and error handling

- Bucket selection is restored only when it still exists and is writable; otherwise the Picker falls back to All buckets.
- Opening or closing the Picker does not reset the remembered bucket.
- The Add Links action is disabled while links are being submitted.
- A submission continues after an individual link fails so the final summary can report both successes and failures.
- Network and server errors count as failed links and surface through the final summary.
- Empty input does not submit and shows a concise validation message.
- If All buckets remains selected, submission is blocked until the user chooses a writable bucket or selects Use Inbox.

## Verification

- Test the shared flow in a desktop browser at `/picker`.
- Test the same flow in the Tauri-hosted Picker.
- Test the mobile-width layout and bucket persistence across page reloads/open-close cycles.
- Verify All buckets blocks submission and Use Inbox selects the valid destination.
- Verify mixed success/failure input produces the expected single summary and clears the input.
- Verify the existing search, bucket selection, and copy/paste behavior remain unchanged.
- Run the web production build and relevant Rust checks.

## Out of scope

- A new batch API endpoint.
- Native-only Tauri commands for adding links.
- Per-link result history or retry controls after the summary is dismissed.
