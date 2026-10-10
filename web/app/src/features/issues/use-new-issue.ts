import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { useNow } from "../../shared/use-now.ts";
import { useMakeIssue } from "./use-make-issue.ts";
import { useNewIssueValues, type NewIssueValuesState } from "./use-new-issue-values.ts";

/** What the new-issue form needs: its values and draft, and what making the issue does. */
export interface NewIssue extends NewIssueValuesState {
  readonly now: number;
  /** Makes the issue; the dialog is done, or starts again with the same project and type. */
  readonly create: (another: boolean, onMade: (key: string, project: string) => void) => void;
  readonly pending: boolean;
  readonly error: unknown;
  /** The issue ID of the last issue made with *Create and start another*. */
  readonly lastKey: string | null;
}

/** The logic of the form of a new issue: its values, and the two ways to make the issue. */
export function useNewIssue(): NewIssue {
  const state = useNewIssueValues();
  const [lastKey, setLastKey] = useState<string | null>(null);
  const shortName = state.project?.shortName;
  const make = useMakeIssue(shortName);
  const now = useNow();
  return {
    ...state,
    now,
    lastKey,
    change: (next) => {
      setLastKey(null);
      state.change(next);
    },
    create: (another, onMade) => {
      make.mutate(state.values, {
        onSuccess: (made: Schema<"IssueItem">) => {
          const key = made.key ?? "";
          if (another) {
            state.startAgain();
            setLastKey(key);
          } else {
            state.forgetDraft();
            onMade(key, shortName ?? "");
          }
        },
      });
    },
    pending: make.isPending,
    error: make.isError ? make.error : null,
  };
}
