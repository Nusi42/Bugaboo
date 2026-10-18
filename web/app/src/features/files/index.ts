import "@bugaboo/messages/parts/files";
import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

// Loaded when an issue is first opened, so the first page stays small (coding-standards section 6).
const FilesBlock = lazyRouteComponent(() => import("./FilesBlock.tsx"), "FilesBlock");

/** Puts *Files* into its slot of the issue page, after the related issues (M4 WP-05). */
export function registerFileBlocks(): void {
  addIssueBlock(issueSlots.files, {
    id: "files",
    order: 0,
    component: FilesBlock,
  });
}

export { ChosenFilesPicker } from "./ChosenFilesPicker.tsx";
export { useChosenFiles, type ChosenFiles } from "./chosen-files.ts";
export { useAddFiles } from "./transfers.ts";
export { CommentFiles } from "./CommentFiles.tsx";
