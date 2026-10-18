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
- **Who may delete:** whoever may change issues, and the person who added the file while they may still add one there (the rule of comments: the right to change the issue for a file of the issue, the right to comment for a file of a comment). *Undo* follows the same rule.
- Enforced by: TEST-UPLOADS (new in M4 WP-05) and SEC-MATRIX.

### 3. Downloads

- **Signed links:** a download goes through a signed link, valid for 5 minutes and for one person (ATT-07).
- **Checked every time:** every download checks again that the person still sees the issue and the comment, so a link a person passes on, or keeps after losing access, stops working. Enforced by: TEST-UPLOADS and SEC-MATRIX.
- **Headers:** every download is sent with `Content-Disposition: attachment`, `X-Content-Type-Options: nosniff`, and the content security policy `default-src 'none'; frame-ancestors 'none'; sandbox`, which allows nothing and lets no page frame it. Enforced by: SEC-HEADERS (the policy is exactly that, and no other code sets the policy header).
- **No HTML or SVG in the page:** SVG, HTML and every other type the browser could run are never shown inside Bugaboo, only downloaded.

### 4. Previews

- Images and PDFs get a preview: a small image the server makes (ATT-04). Nothing else does.
- **In a separate process:** a preview is made by `bugaboo preview`, a child process with limits on memory, time and pixels (a named constant each). It is given nothing of the server's: an empty environment, the previews work folder as its working folder and the bytes of the file on its standard input (no path); it writes one small PNG on its standard output. **That is not a sandbox:** the child runs as the account of the server and so holds that account's rights (if code runs in it, it can read the keys and the database and reach the network). The protection is the separation of the process, the limits on memory, time and pixels, and the small set of decoders; it stops crashes and bombs, not a compromise through a memory-safety bug in a decoder. Hardening (landlock or seccomp on Linux, a separate user, a restricted token on Windows) is the follow-up M4-F121, and the risk is among the accepted risks of the security architecture. A file that is too big, takes too long or crashes the child gets no preview, and the server goes on.
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

## Built in M4 WP-05

What the build decided where this record left a choice:

- **Protocol.** The upload protocol says `offset`: a part names where it starts, and the server answers with the offset of its copy (no counter that grows, ARCH-NO-COUNTERS). A browser that lost the connection asks where the server stands and goes on from there; *Try again* goes on with the same upload, and a refusal or *Close* throws it away. A person who loses access in the middle of an upload ends it: the next part or the finish is refused and its bytes and row are thrown away at once.
- **Limits.** A person has at most 20 open uploads that declare at most four times the biggest file together (`OPEN_UPLOADS_MAX`, `OPEN_UPLOADS_BYTES_MAX`; the count, the sum and the insert are one step); an issue holds at most 200 files, those in the trash included, judged when an upload starts and again when it finishes; a part is refused (`file.no_room`, 507) when the disk would be left with less than 1 GiB (`DISK_KEPT_FREE_BYTES`).
- **Names.** A name loses the dots and spaces at its end (Windows drops them) and is refused with a control or format character in it (U+202E turns `gpj.exe` into `exe.jpg` on the screen), before the list of blocked types is read.
- **Downloads.** A download is the bytes of the file, always sent as `application/octet-stream`; a preview is always `image/png`. The type found in the content is only shown to people. The key of the signed links is the key of the CSRF tokens, kept apart by a name of its own that every signature starts with (`bugaboo.link.v1`).
- **Previews.** Every attachment has its own preview status (`pending`, `ready` or `without`) and its own job `files.preview`. A file that is stored already is not told apart from a new one: finishing it answers exactly as for a new file, with no preview and no link; the job of the attachment takes an existing preview of the same content only when it runs, as the attachment's own status, and makes none again. The page reads the list again while a preview is pending.
- **Jobs.** Only the clean-up is daily (ADR-0017): `files.clean-up` deletes unfinished uploads after one day, trashed attachments after 30 days, and files nothing points to. A file is held (`FileStore::hold`) from putting its bytes into the store until its attachment is there, and the clean-up holds it while it deletes the row, by one conditional statement, and then the bytes, so it cannot take the bytes of a file that is being added.
- **The child process.** It starts with an empty environment, the previews work folder as its working folder, the bytes of the file on standard input and one PNG on standard output; the server reads at most one byte more than a preview may have and writes the file itself. Before it reads a byte it limits itself: pixels (40 million, from the header), what the decoder may allocate (512 MiB), input size (the biggest file the settings allow), and memory. The memory limit is `MAX_CHILD_MEMORY_BYTES` (1 GiB) on the heap on every system, by the `cap` allocator that the program runs with (a refused allocation ends the child, and the server reads that as no preview), and on Linux also `MAX_CHILD_ADDRESS_SPACE_BYTES` (2 GiB) of address space by `setrlimit`. On Windows there is no limit of the address space: no safe crate sets the memory limit of a job object, and `unsafe` code is forbidden; the heap limit counts what Rust code asks for, not stacks, code or what the system maps. The server stops the child after 20 seconds. The test `att_04_a_pdf_bomb_costs_the_child_its_memory_limit_and_the_preview_only` runs the real program on a PDF whose picture unpacks to one and a half times the limit.
- **Accepted risk: the time of the preview (§4).** The preview of a file that is stored already is taken over by the job of its attachment as soon as that job runs, while the preview of a new file is made first, which takes longer. A person who adds a picture or a PDF and watches how soon its preview appears can tell, a little, that the same content was stored before, by somebody they cannot know. The statuses and links in every answer are the same, no delay is added, and the risk is accepted: the person already holds the content, nothing of the other attachment is shown, and a delay as long as the slowest preview would make every picture wait.
- **Accepted risk.** Finishing a file that is stored already is a little faster than finishing a new one (the bytes are not moved), so a person who measures the time can tell that the same content was stored before, by somebody they cannot know. Nothing else tells it: the answers are the same. Recorded in the security chapter, accepted because the content is already known to the person who adds it, nothing of the other attachment is shown, and equal time would mean reading and writing the file again for nothing.

## Enforced by

TEST-UPLOADS (new), SEC-MATRIX (attachment routes and downloads), SEC-HEADERS (download headers), ARCH-DEPS (the port), DATA-MARKS, DEP-LICENSES and DEP-REGISTER (`infer`, `image`, `hayro`), and review (no route serves `files/` directly).
