import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../../shared/api.ts";
import { messageOf } from "../../../shared/errors.ts";
import { refreshKeys } from "./access-keys.ts";
import { bodyOf, type Draft } from "./draft.ts";
import { keyEndOf } from "./expiry-day.ts";

/** Asking the server to make the key. */
export interface CreateKey {
  /** Sends what the person filled in, unless the day is no day (then `wrongDay` is set and nothing is sent). */
  readonly send: (draft: Draft) => void;
  /** The day that was filled in is no day: a problem of the form, never the default in its place. */
  readonly wrongDay: boolean;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
  readonly isPending: boolean;
}

/** Making a key: when it is made the list reads again and `onMade` gets the whole key, once. */
export function useCreateKey(
  onMade: (secret: string) => void,
  rules: Schema<"AccessKeyRulesBody">,
): CreateKey {
  const [wrongDay, setWrongDay] = useState(false);
  // The answer holds the secret of the key: it is dropped from the cache of requests as soon as
  // nobody looks at it (`gcTime: 0`), and the request is reset once the secret has been handed on.
  const create = api.query.useMutation("post", "/api/v1/access-keys", {
    gcTime: 0,
    onSuccess: (made: Schema<"CreatedAccessKeyBody">) => {
      refreshKeys();
      onMade(made.secret);
    },
  });
  return {
    wrongDay,
    send: (draft) => {
      const end = keyEndOf(draft.day, rules);
      setWrongDay(!end.valid);
      if (!end.valid) {
        return;
      }
      create.mutate(
        { body: bodyOf(draft, end.expiresAt) },
        {
          onSuccess: () => {
            create.reset();
          },
        },
      );
    },
    problem: create.isError ? messageOf(create.error) : null,
    isPending: create.isPending,
  };
}
