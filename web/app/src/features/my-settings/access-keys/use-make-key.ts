import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { messageOf } from "../../../shared/errors.ts";
import { refreshKeys } from "./access-keys.ts";
import { bodyOf, type Draft } from "./draft.ts";

/** Asking the server to make the key. */
export interface MakeKey {
  /** Sends what the person filled in. */
  readonly send: (draft: Draft) => void;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
  readonly isPending: boolean;
}

/** Making a key: when it is made the list reads again and `onMade` gets the whole key, once. */
export function useMakeKey(onMade: (secret: string) => void): MakeKey {
  const make = api.query.useMutation("post", "/api/v1/access-keys", {
    onSuccess: (made: Schema<"CreatedAccessKeyBody">) => {
      refreshKeys();
      onMade(made.secret);
    },
  });
  return {
    send: (draft) => {
      make.mutate({ body: bodyOf(draft) });
    },
    problem: make.isError ? messageOf(make.error) : null,
    isPending: make.isPending,
  };
}
