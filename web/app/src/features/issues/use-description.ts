import { dataOf } from "@bugaboo/api-client";
import { useCallback, useState } from "react";

import { api } from "../../shared/api.ts";
import { useLongText, type LongText } from "../../shared/editing/index.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { IssueView } from "./slots.ts";

/** The description of an issue as a long text being edited, and when it was last saved. */
export interface Description extends LongText {
  /** When the last save of this screen went through, as a moment in time. */
  readonly savedAt: string | null;
}

/**
 * The description of the issue as a long text that is edited under a lock (ISS-17): it is saved
 * by `PUT /issues/{issue}/description` with the token of the lock and the version the person
 * started from, and the page reads the issue again afterwards.
 */
export function useDescription(issue: IssueView, address: string): Description {
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const id = issue.id ?? address;
  const save = useCallback(
    async (input: {
      readonly text: string;
      readonly version: number;
      readonly lockToken: string;
    }) => {
      dataOf(
        await api.client.PUT("/api/v1/issues/{issue}/description", {
          params: { path: { issue: id } },
          body: { text: input.text, version: input.version, lock_token: input.lockToken },
        }),
      );
      setSavedAt(new Date().toISOString());
      void queryClient.invalidateQueries({ queryKey: queryKeys.issues.lists() });
    },
    [id],
  );
  const refresh = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.issues.one(address) });
  }, [address]);
  const text = useLongText({
    kind: "issue",
    id,
    part: "description",
    saved: issue.description ?? "",
    version: issue.version ?? 1,
    save,
    refresh,
  });
  return { ...text, savedAt };
}
