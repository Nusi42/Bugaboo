import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { NO_GROUP, refreshLabels, type Label } from "./labels.ts";

/** What is filled in for a label so far, and the request that saves it. */
export function useLabelDraft(label: Label) {
  const [name, setName] = useState(label.name);
  const [color, setColor] = useState(label.color);
  const [group, setGroup] = useState(label.label_group ?? NO_GROUP);
  const change = api.query.useMutation("patch", "/api/v1/labels/{label}", {
    onSuccess: refreshLabels,
  });
  return {
    name,
    setName,
    color,
    setColor,
    group,
    setGroup,
    saving: change.isPending,
    saved: change.isSuccess,
    error: change.isError ? messageOf(change.error) : null,
    /** Saves the change, on the version that was looked at. */
    save: (): void => {
      change.mutate({
        params: { path: { label: label.id } },
        body: {
          version: label.version,
          name: name.trim(),
          color,
          label_group: group === NO_GROUP ? null : group,
        },
      });
    },
  };
}
