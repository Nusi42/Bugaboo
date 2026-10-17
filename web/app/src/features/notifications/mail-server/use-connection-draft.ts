import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../../shared/api.ts";
import { messageOf } from "../../../shared/errors.ts";
import { type MySettings } from "../../../shared/my-settings.ts";
import { saveStateOf } from "../../../shared/optimistic.ts";
import { refreshMailServer } from "../../../shared/query-keys.ts";
import { changesOf, draftOf, type Changes, type Draft } from "./connection-draft.ts";
import { useSetMailPassword } from "./use-mail-server.ts";

/** Puts the settings one after the other, on purpose: the order is what keeps the new password. */
async function putInOrder(changes: Changes["changes"]): Promise<void> {
  const [first, ...rest] = changes;
  if (first === undefined) {
    return;
  }
  const [key, value] = first;
  await api.client.PUT("/api/v1/settings/{key}", { params: { path: { key } }, body: { value } });
  await putInOrder(rest);
}

/**
 * Saves the changed settings of the mail server one after the other, and the new password last:
 * a new host, port or user name takes the old password away in the same change, so a password
 * sent before them would be lost, and one sent after them belongs to the new server.
 */
function useSaveConnection() {
  const sendPassword = useSetMailPassword();
  return useMutation({
    mutationFn: async (found: {
      readonly changes: Changes["changes"];
      readonly password: string;
    }) => {
      await putInOrder(found.changes);
      if (found.password !== "") {
        await sendPassword.mutateAsync({ body: { password: found.password } });
      }
    },
    onSettled: refreshMailServer,
  });
}

/**
 * What the form of the mail server keeps while the person types: the draft, the problem with the
 * port, and what saving does.
 */
export function useConnectionDraft(settings: MySettings) {
  const [draft, setDraft] = useState<Draft>(draftOf(settings));
  const [problem, setProblem] = useState<string>();
  const saving = useSaveConnection();
  return {
    draft,
    problem,
    saveState: saveStateOf(saving.status),
    saveProblem: saving.isError ? messageOf(saving.error) : null,
    field: (name: keyof Draft) => (next: string) => {
      setDraft({ ...draft, [name]: next });
      setProblem(undefined);
    },
    save: (): void => {
      const found = changesOf(settings, draft);
      setProblem(found.problem);
      if (found.problem === undefined) {
        // The typed password stays in the box until the save has worked: a refused save keeps it.
        saving.mutate(
          { changes: found.changes, password: draft.password },
          {
            onSuccess: () => {
              setDraft((now) => ({ ...now, password: "" }));
            },
          },
        );
      }
    },
  };
}
