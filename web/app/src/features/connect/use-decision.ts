import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import type { ConnectRequest } from "./request.ts";

/** What the person decided. */
export interface Decision {
  /** *Allow* or *Cancel*. */
  readonly allow: boolean;
  /** What is allowed. */
  readonly scope: Schema<"KeyChoice">;
  /** The projects it is limited to; none for every project. */
  readonly projects: readonly string[];
}

/** Sending the decision, and going back to the app with the answer. */
export interface Deciding {
  readonly send: (decision: Decision) => void;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
  /** The answer is in and the browser is on its way to the app. */
  readonly leaving: boolean;
  readonly isPending: boolean;
}

/**
 * *Allow* and *Cancel*: the server makes the grant (or not) and says where the app waits for the
 * answer; the browser goes there. The code in that address is for the app alone, so the answer is
 * dropped from the cache of requests at once (`gcTime: 0`).
 */
export function useDecision(request: ConnectRequest): Deciding {
  const [leaving, setLeaving] = useState(false);
  const decide = api.query.useMutation("post", "/api/v1/oauth/authorization", {
    gcTime: 0,
    onSuccess: (answer: Schema<"DecisionAnswerBody">) => {
      setLeaving(true);
      window.location.assign(answer.redirect_to);
    },
  });
  return {
    send: ({ allow, scope, projects }) => {
      decide.mutate({
        body: { request, allow, scope, projects: [...projects] },
      });
    },
    problem: decide.isError ? messageOf(decide.error) : null,
    leaving,
    isPending: decide.isPending,
  };
}
