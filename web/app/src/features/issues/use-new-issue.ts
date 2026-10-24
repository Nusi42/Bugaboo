import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { useNow } from "../../shared/use-now.ts";
import { useFormLayout, type FormView } from "../layouts/index.ts";
import { forForm } from "./new-issue-form.ts";
import { useCreateIssue } from "./use-create-issue.ts";
import { useNewIssueValues, type NewIssueValuesState } from "./use-new-issue-values.ts";

/** What the new-issue form needs: its values and draft, and what making the issue does. */
export interface NewIssue extends NewIssueValuesState {
  readonly now: number;
  /** Which fields the layout of the form asks this person for. */
  readonly layout: FormView;
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
  const create = useCreateIssue(shortName);
  const now = useNow();
  const layout = useFormLayout({
    project: shortName,
    issueType: state.values.issueType === "" ? undefined : state.values.issueType,
    kind: "new-issue",
  });
  return {
    ...state,
    now,
    layout,
    lastKey,
    change: (next) => {
      setLastKey(null);
      // What was refused is about the values before this change.
      create.reset();
      state.change(next);
    },
    create: (another, onMade) => {
      create.mutate(forForm(state.values, layout.shows), {
        onSuccess: (made: Schema<"IssueItem">) => {
          const key = made.readable_id ?? "";
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
    pending: create.isPending,
    error: create.isError ? create.error : null,
  };
}
