import type { Schema } from "@bugaboo/api-client";

/** An issue as the server shows it in a list: what the viewer may see of it in its own project. */
export type ShownIssue = Schema<"IssueRef">;
