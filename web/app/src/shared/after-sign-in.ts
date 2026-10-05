import type { Schema } from "@bugaboo/api-client";
import { useNavigate } from "@tanstack/react-router";

import { paths } from "./paths.ts";
import { forgetPerson } from "./session.ts";

/** What the server answers when somebody signed in or made an account. */
export type SignedIn = Schema<"SignedInBody">;

/**
 * What to do with the answer of a sign-in, whichever way it came: a password, a passkey, an
 * invitation, signing up or the setup. A person whose account asks for a second step goes to
 * that page first; everybody else goes to the start page.
 */
export function useAfterSignIn(): (answer: SignedIn) => void {
  const navigate = useNavigate();
  return (answer) => {
    const second = answer.second_step;
    if (second !== null && second !== undefined) {
      void navigate({
        to: paths.secondStep,
        search: { app: second.code_app, passkey: second.passkey },
      });
      return;
    }
    if (answer.person !== null && answer.person !== undefined) {
      forgetPerson();
      void navigate({ to: paths.home });
    }
  };
}
