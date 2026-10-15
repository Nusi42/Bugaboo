import { dataOf, descriptionEditPart } from "@bugaboo/api-client";
import { useCallback, useState } from "react";

import { api } from "../../shared/api.ts";
import { useLongText, type LongText } from "../../shared/editing/index.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { versionOf } from "./issue-values.ts";
import type { IssueView } from "./slots.ts";

/** The description of an issue as a long text being edited, and when it was last saved. */
export interface Description extends LongText {
  /** When the last save of this screen went through, as a moment in time. */
  readonly savedAt: string | null;
}

/** Saves a long text of an issue under the lock; rejects with the error of the server. */
export type SaveLongText = (input: {
  readonly text: string;
  readonly version: number;
  readonly lockToken: string;
}) => Promise<void>;

/** Which long text of an issue: its part, the text as saved now, and how to save it. */
export interface IssueLongTextOptions {
  /** The name of the part the lock and the drafts are about, like `description`. */
  readonly part: string;
  /** The text as saved now. */
  readonly saved: string;
  /** Saves it; has to stay the same while the text is on screen. */
  readonly save: SaveLongText;
}

/**
 * A long text of the issue that is edited under a lock (ISS-17, CF-01): the description, or a long
 * text own field. The page reads the issue again after a save.
 */
export function useIssueLongText(
  issue: IssueView,
  address: string,
  which: IssueLongTextOptions,
): Description {
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const id = issue.id ?? address;
  const { save: saveText } = which;
  const save = useCallback<SaveLongText>(
    async (input) => {
      await saveText(input);
      setSavedAt(new Date().toISOString());
      void queryClient.invalidateQueries({ queryKey: queryKeys.issues.lists() });
    },
    [saveText],
  );
  const refresh = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.issues.one(address) });
  }, [address]);
  const text = useLongText({
    kind: "issue",
    id,
    part: which.part,
    saved: which.saved,
    version: versionOf(issue),
    save,
    refresh,
  });
  return { ...text, savedAt };
}

/**
 * The description of the issue as a long text that is edited under a lock (ISS-17): it is saved
 * by `PUT /issues/{issue}/description` with the token of the lock and the version the person
 * started from.
 */
export function useDescription(issue: IssueView, address: string): Description {
  const id = issue.id ?? address;
  const save = useCallback<SaveLongText>(
    async (input) => {
      dataOf(
        await api.client.PUT("/api/v1/issues/{issue}/description", {
          params: { path: { issue: id } },
          body: { text: input.text, version: input.version, lock_token: input.lockToken },
        }),
      );
    },
    [id],
  );
  return useIssueLongText(issue, address, {
    part: descriptionEditPart,
    saved: issue.description ?? "",
    save,
  });
}
