import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { formatDateTime } from "../../shared/format.ts";
import { personNameInside } from "../../shared/person.ts";
import { formatSize } from "./size.ts";

/** A file of an issue or a comment, as the reader sees it. */
export type FileItem = Schema<"AttachmentItem">;

/** The line under the name of a file: how big it is, who added it and when. */
export function metaOf(item: FileItem): string {
  return t("files.added", {
    size: formatSize(item.size),
    who: personNameInside(item.added_by),
    when: formatDateTime(item.added_at),
  });
}
