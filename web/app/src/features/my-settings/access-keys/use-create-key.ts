import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../../shared/api.ts";
import { messageOf } from "../../../shared/errors.ts";
import { refreshKeys } from "./access-keys.ts";
import { bodyOf, type Draft } from "./draft.ts";
import { keyEndOf, stillWrongDay } from "./expiry-day.ts";

/** Asking the server to make the key. */
export interface CreateKey {
  /** Sends what the person filled in, unless the day is no day (then `wrongDay` is set and nothing is sent). */
  readonly send: (draft: Draft) => void;
  /** The day in the box is the one that was sent as no day: a problem of the form, never the default in its place. It is gone once the person changes the day. */
  readonly wrongDay: (day: string | null) => boolean;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
  readonly isPending: boolean;
}

/** Making a key: when it is made the list reads again and `onMade` gets the whole key, once. */
export function useCreateKey(
  onMade: (secret: string) => void,
  rules: Schema<"AccessKeyRulesBody">,
): CreateKey {
  // The text of the day that was no day; the problem is that text, so another day ends it.
  const [wrongText, setWrongText] = useState<string | null>(null);
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
    wrongDay: (day) => stillWrongDay(wrongText, day),
    send: (draft) => {
      const end = keyEndOf(draft.day, rules);
      setWrongText(end.valid ? null : draft.day);
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
