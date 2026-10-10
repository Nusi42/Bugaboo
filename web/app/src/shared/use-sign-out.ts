import { useNavigate } from "@tanstack/react-router";

import { api } from "./api.ts";
import { paths } from "./paths.ts";
import { signOutOnPurpose } from "./session.ts";

/** Signing out of this browser: the session ends and the person is taken to the sign-in page. */
export interface SignOut {
  /** Ends the session. */
  readonly signOut: () => void;
  readonly isPending: boolean;
  /** What went wrong, if the server could not end the session. */
  readonly error: unknown;
}

/** The one way the app signs a person out, for the frame of the app and for the screens without it. */
export function useSignOut(): SignOut {
  const navigate = useNavigate();
  const signOut = api.query.useMutation("post", "/api/v1/auth/sign-out", {
    onSuccess: () =>
      signOutOnPurpose(() => navigate({ to: paths.signIn, search: { reason: "signed-out" } })),
  });
  return {
    signOut: () => {
      signOut.mutate({});
    },
    isPending: signOut.isPending,
    error: signOut.isError ? signOut.error : null,
  };
}
