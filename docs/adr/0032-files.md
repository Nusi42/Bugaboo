# ADR-0032: Files: stored by content, checked from their content, previews in a separate process

Status: **Accepted** · 2026-10-16 · refines ATT-01…ATT-04, ATT-07, SEC-13; decisions M4-Q3, M4-Q4

## Context

M4 lets people add files to issues and comments (ATT-01). A file is the most dangerous thing a person can give the server. It may claim to be an image and be a script, it may be an SVG or HTML page that runs in the browser, it may be a small file that unpacks to gigabytes, and it may be built to crash the program that reads it. The data folder already has an empty `files/` folder (architecture §6), and `bb-files` is an empty crate.

## Decision

### 1. Storage

- A file is stored once, by the SHA-256 of its content, under `files/` in the data folder, outside any web root (ATT-02). The server never serves that folder directly.
- An **attachment** points to a file and to its issue or comment. It records who added it, its name and its real type.
- A file is deleted when no attachment, document version or keep rule points to it any more. A daily job (ADR-0017) does it. Enforced by: a test of the job on both databases (M4 WP-05).
- A deleted attachment goes to the trash with *Undo*, like an issue (M4-Q3).
- Storage sits behind a port (`FileStore` in `bb-app`), so S3-compatible storage (ATT-06, V1) is a second adapter later. Enforced by: ARCH-DEPS (`bb-app` never imports `bb-files`).

### 2. Uploads

- **Parts:** an upload goes in parts. Bugaboo's own small protocol: start an upload (size, name), send parts with their offset, finish. An interrupted upload goes on from the last part the server has. An unfinished upload is deleted after a day. No extra library.
- **Body limit:** each upload route sets its own body limit for a part. All other routes keep the 1 MiB limit (M1-F30).
- **Size (M4-Q4):** at most 25 MB per file by default. A project may set less, and the organization up to 100 MB. A file over the limit is refused before its parts are stored.
- **Type:** the real type is read from the content (`infer`), never trusted from the name or the browser (ATT-03). All types are accepted. The organization may list types it never accepts (for example `.exe`).
- **Who may add:** a person who may change the issue (Guests on the issues they reported), or a person who may comment, for files on their own comment (M4-Q3).
- **Who may delete:** the person who added the file, or a person with *Change issues*.
- Enforced by: TEST-UPLOADS (new in M4 WP-05) and SEC-MATRIX.

### 3. Downloads

- **Signed links:** a download goes through a signed link, valid for 5 minutes and for one person (ATT-07).
- **Checked every time:** every download checks again that the person still sees the issue and the comment, so a link a person passes on, or keeps after losing access, stops working. Enforced by: TEST-UPLOADS and SEC-MATRIX.
- **Headers:** every download is sent with `Content-Disposition: attachment`, `X-Content-Type-Options: nosniff`, and a content security policy that allows nothing. Enforced by: SEC-HEADERS.
- **No HTML or SVG in the page:** SVG, HTML and every other type the browser could run are never shown inside Bugaboo, only downloaded.

### 4. Previews

- Images and PDFs get a preview: a small image the server makes (ATT-04). Nothing else does.
- **In a separate process:** a preview is made by `bugaboo preview`, a child process with limits on memory, time and pixels (a named constant each). It reads one file and writes one small image of a fixed safe type. A file that is too big, takes too long or crashes the child gets no preview, and the server goes on.
- **As a job:** previews are made in a job (ADR-0017), at most a few at once (the worker pool of ADR-0017).
- **Libraries:** `image` for images and `hayro` for PDFs, both pure Rust. `hayro`'s license is checked before it is added (DEP-LICENSES).
- **Served like files:** a preview is shown in the page as an image from our own origin, through a signed link like the file itself.
- Enforced by: TEST-UPLOADS (a decompression bomb, a broken PDF and a huge image each end with no preview and a running server).

### 5. Personal data

- An attachment records who added it, and a file's content is personal data like a comment.
- Files of a deleted issue go with the issue after the trash's 30 days.
- Enforced by: DATA-MARKS.

## Consequences

- The issue page gets a *Files* block. Comments show their files. The many-issues bar does not add files.
- Virus scanning (ATT-05) stays V1: the port has room for a scan step before a file becomes visible.

## Alternatives considered

- **tus or another upload library:** more than we need, and one more dependency in the browser and the server. Rejected for our own three routes.
- **Previews in the server process:** a crafted file could take the server down. Rejected.
- **Files in the database:** backups grow with every screenshot, and PostgreSQL large objects are awkward. Rejected; the database holds only the attachment rows.

## Enforced by

TEST-UPLOADS (new), SEC-MATRIX (attachment routes and downloads), SEC-HEADERS (download headers), ARCH-DEPS (the port), DATA-MARKS, DEP-LICENSES and DEP-REGISTER (`infer`, `image`, `hayro`), and review (no route serves `files/` directly).
