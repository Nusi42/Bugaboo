import { useState } from "react";

import { type MySettings } from "../../../shared/my-settings.ts";
import { changesOf, draftOf, type Draft, type SettingEdits } from "./connection-draft.ts";
import { useSetMailPassword } from "./use-mail-server.ts";

/**
 * What the form of the mail server keeps while the person types: the draft, the problem with the
 * port, and what saving does (the changed settings, and the password on its own).
 */
export function useConnectionDraft(settings: MySettings, edits: SettingEdits) {
  const [draft, setDraft] = useState<Draft>(draftOf(settings));
  const [problem, setProblem] = useState<string>();
  const sendPassword = useSetMailPassword();
  return {
    draft,
    problem,
    passwordProblem: sendPassword.isError ? sendPassword.error : undefined,
    field: (name: keyof Draft) => (next: string) => {
      setDraft({ ...draft, [name]: next });
      setProblem(undefined);
    },
    save: (): void => {
      const found = changesOf(settings, draft);
      setProblem(found.problem);
      if (found.problem !== undefined) {
        return;
      }
      for (const [key, value] of found.changes) {
        edits.change({ kind: "change", key, value });
      }
      if (draft.password !== "") {
        sendPassword.mutate({ body: { password: draft.password } });
        setDraft({ ...draft, password: "" });
      }
    },
  };
}
