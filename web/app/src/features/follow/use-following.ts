import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

const reasonMessage = {
  reported: "followers.reason-reported",
  assigned: "followers.reason-assigned",
  commented: "followers.reason-commented",
  chose: "followers.reason-chose",
} as const;

/** What the person's own choice to follow an issue is, and the way to turn it round. */
export interface Following {
  /** The choice was read: until it is, `following` says nothing. */
  readonly known: boolean;
  readonly following: boolean;
  /** Why the person follows, in words, when they do. */
  readonly why: string | undefined;
  readonly pending: boolean;
  /** What went wrong turning it on, in words, if anything. */
  readonly problem: string | null;
  readonly turn: () => void;
}

/**
 * The person's own choice to follow the issue: whether they do, why, and the way to turn it round. `asked` is false
 * where the choice is not needed yet (a row of a list before it is touched): nothing is read until it is.
 */
export function useFollowing(issue: string, asked = true): Following {
  const state = api.query.useQuery(
    "get",
    "/api/v1/issues/{issue}/follow",
    { params: { path: { issue } } },
    { enabled: asked },
  );
  const follow = api.query.useMutation("put", "/api/v1/issues/{issue}/follow", {
    onSuccess: refreshHistory,
  });
  const leave = api.query.useMutation("delete", "/api/v1/issues/{issue}/follow", {
    onSuccess: refreshHistory,
  });
  const params = { params: { path: { issue } } } as const;
  const following = state.data?.following === true;
  const reason = state.data?.reason;
  return {
    known: state.isSuccess,
    following,
    // Why the person follows: read with the button that turns it round, for those who cannot point at it.
    why: reason === undefined || reason === null ? undefined : t(reasonMessage[reason]),
    pending: follow.isPending || leave.isPending,
    problem: follow.error === null ? null : messageOf(follow.error),
    turn: () => {
      if (following) {
        leave.mutate(params);
      } else {
        follow.mutate(params);
      }
    },
  };
}
