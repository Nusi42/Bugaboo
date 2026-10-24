import { commentEditPart, dataOf } from "@bugaboo/api-client";
import { useCallback, useEffect, useRef } from "react";

import { api } from "../../shared/api.ts";
import { useLongText, type LongText } from "../../shared/editing/index.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** What the editor of a comment needs to know about the comment. */
export interface CommentSource {
  readonly comment: string;
  readonly version: number;
  /** The text as saved now. */
  readonly text: string;
}

/**
 * The text of a comment as a long text being edited (ISS-05, ADR-0019), the way the description is:
 * the lock, the drafts of the browser and of the server, and the three save states. The editor
 * opens as the screen is mounted, and `onDone` is called when the text was saved or thrown away.
 */
export function useCommentText(source: CommentSource, onDone: () => void): LongText {
  const { comment, version, text } = source;
  const save = useCallback(
    async (input: {
      readonly text: string;
      readonly version: number;
      readonly lockToken: string;
    }) => {
      dataOf(
        await api.client.PUT("/api/v1/comments/{comment}", {
          params: { path: { comment } },
          body: { version: input.version, lock_token: input.lockToken, text: input.text },
        }),
      );
    },
    [comment],
  );
  const refresh = useCallback(() => {
    refreshHistory();
    return Promise.resolve();
  }, []);
  const long = useLongText({
    kind: "comment",
    id: comment,
    part: commentEditPart,
    saved: text,
    version,
    save,
    refresh,
  });
  const { open } = long;
  useEffect(() => {
    open();
  }, [open]);
  // Done is when the box has been open and is closed again (saved or thrown away).
  const wasOpen = useRef(false);
  const closed = long.phase.kind === "reading";
  useEffect(() => {
    if (!closed) {
      wasOpen.current = true;
    } else if (wasOpen.current) {
      onDone();
    }
  }, [closed, onDone]);
  return long;
}
