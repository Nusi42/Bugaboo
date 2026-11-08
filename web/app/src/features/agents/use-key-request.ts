import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { keyEndOf, type KeyDraft } from "../my-settings/index.ts";
import { refreshAgents, type AgentKey } from "./agents.ts";

/** Asking the server to create an agent key, or a new one in the place of an old one. */
export function useKeyRequest(
  agent: string,
  renewing: AgentKey | null,
  onMade: (secret: string) => void,
) {
  const [wrongDay, setWrongDay] = useState(false);
  const create = api.query.useMutation("post", "/api/v1/agents/{agent}/keys", { gcTime: 0 });
  const renew = api.query.useMutation("post", "/api/v1/agents/{agent}/keys/{access_key}/renew", {
    gcTime: 0,
  });
  const done = {
    onSuccess: (made: Readonly<{ secret: string }>) => {
      refreshAgents();
      onMade(made.secret);
    },
  };
  const send = (draft: KeyDraft, rules: Schema<"AccessKeyRulesBody">): void => {
    const end = keyEndOf(draft.day, rules);
    setWrongDay(!end.valid);
    if (!end.valid) {
      return;
    }
    if (renewing === null) {
      create.mutate(
        {
          params: { path: { agent } },
          body: { password: draft.password, name: draft.name.trim(), expires_at: end.expiresAt },
        },
        done,
      );
    } else {
      renew.mutate(
        {
          params: { path: { agent, access_key: renewing.id } },
          body: { password: draft.password, expires_at: end.expiresAt },
        },
        done,
      );
    }
  };
  return {
    send,
    wrongDay,
    pending: create.isPending || renew.isPending,
    failure: create.error ?? renew.error,
  };
}
