# WP-05 — Files on issues and comments: upload, safe storage, type checks, previews

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ATT-01, ATT-02, ATT-03, ATT-04, ATT-07, SEC-13 (uploads); UX design §4.4 (*Files*); ADR-0032, ADR-0017; follow-up M1-F30.

## Goal

People add screenshots and files to issues and comments by dragging, pasting or choosing them. Bugaboo stores them safely, shows previews of images and PDFs, and lets only people who see the issue download them. No file can run in a browser or take the server down.

## Owns

- `crates/bb-files/**` (the adapter: storage by content hash, type check, the preview child).
- The port `FileStore` in `bb-app` and `crates/bb-app/src/files/**` (new).
- `crates/bb-store/src/attachments.rs` (new) and migration `m0016`.
- `crates/bb-api/src/api/files*` (new): the upload routes with their own body limit, signed download links, the download headers.
- The `bugaboo preview` subcommand in `bb-server`.
- The settings of the limits per organization and project.
- The *Files* block of the issue page (a slot in `features/issues/slots.ts`) and files on comments.
- The file parts in `design/theme.kdl`.
- TEST-UPLOADS (planned → active).
- Third-party rows for `infer`, `image`, `hayro` (check `hayro`'s license first; if it is not on the allow list, stop and ask the lead).
- `userDocs/files/*` (new).

## Tasks

1. **Storage (ADR-0032 §1):**
   - Files by SHA-256 under `files/`; attachments point to them.
   - A daily job deletes files nothing points to.
   - Deleted attachments go to the trash with *Undo*.
2. **Uploads (§2):**
   - **Parts:** start, parts with offsets, finish; an interrupted upload goes on.
   - **Unfinished uploads** are deleted after a day.
   - **Type and size:** the real type comes from `infer`. The limits are 25 MB by default; a project may set less and the organization up to 100 MB. The organization may also list blocked types.
   - **Permissions:** who may add and who may delete, as M4-Q3 says.
3. **Downloads (§3):**
   - Signed links for one person and 5 minutes, and a fresh access check on every download.
   - The headers: `attachment`, `nosniff`, a policy that allows nothing.
   - SVG and HTML are only downloaded.
4. **Previews (§4):**
   - `bugaboo preview` as a child process with limits on memory, time and pixels (named constants), run from a job with the worker pool of ADR-0017.
   - `image` for images, `hayro` for the first page of a PDF; one small safe image out.
5. **Screens (UX §4.4):**
   - **The *Files* block** on the issue page and files under comments.
   - **Adding:** drag, paste and *Upload*; progress where the file will be, and it goes on after a lost connection.
   - **Refusals** with the limit in words.
   - **Preview larger** on click; *Download*; *Delete* with *Undo*.
   - Four themes, computer and phone, axe.

## Acceptance tests (write first)

- `att_02_a_file_is_stored_once_by_its_content_and_deleted_when_nothing_points_to_it` (both databases).
- `att_01_an_interrupted_upload_goes_on_from_the_last_part`.
- `att_03_*`:
  - `att_03_the_type_comes_from_the_content_not_the_name`;
  - `att_03_a_file_over_the_limit_is_refused_before_it_is_stored`;
  - `att_03_a_blocked_type_is_refused`.
- `att_02_svg_and_html_are_only_downloaded_with_attachment_and_nosniff`.
- `att_04_*`:
  - `att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on`;
  - `att_04_a_broken_pdf_gets_no_preview`;
  - `att_04_a_huge_image_is_refused_by_the_pixel_limit`.
- `att_07_*`:
  - `att_07_a_link_stops_working_after_access_ends`;
  - `att_07_a_link_is_for_one_person_and_five_minutes`.
- SEC-MATRIX rows: a Guest adds files only on issues they reported; a Viewer adds none.
- Playwright: paste a screenshot and see its preview; drag a PDF; a too big file is refused in words; four themes, axe.

## Checks

TEST-UPLOADS (planned → active). SEC-HEADERS covers the download headers.

## Depends on

WP-01 (ADR-0032), WP-02 (the new names, the free migration number).
