import type { Schema } from "@bugaboo/api-client";
import { agentsAtOnceMost } from "@bugaboo/api-client";
import { useState } from "react";

import { keyEndOf, type KeyDraft } from "../my-settings/index.ts";
import { useCreateSeveralAgents } from "./agents.ts";
import type { CreatedAgent } from "./several-agents.ts";

/** Asking the server to create the agents of a list, each with one key, after the password once. */
export function useSeveralRequest(onCreated: (created: readonly CreatedAgent[]) => void) {
  const [wrongDay, setWrongDay] = useState(false);
  const create = useCreateSeveralAgents();
  const send = (
    names: readonly string[],
    draft: KeyDraft,
    rules: Schema<"AccessKeyRulesBody">,
  ): void => {
    const end = keyEndOf(draft.day, rules);
    setWrongDay(!end.valid);
    if (!end.valid || names.length === 0 || names.length > agentsAtOnceMost) {
      return;
    }
    create.mutate(
      {
        body: {
          password: draft.password,
          agents: names.map((name) => ({ name, expires_at: end.expiresAt })),
        },
      },
      {
        onSuccess: (answer: Readonly<{ items: readonly CreatedAgent[] }>) => {
          onCreated(answer.items);
        },
      },
    );
  };
  return { send, wrongDay, pending: create.isPending, failure: create.error };
}
