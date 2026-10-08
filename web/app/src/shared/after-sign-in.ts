import type { Schema } from "@bugaboo/api-client";
import { useNavigate, useRouter } from "@tanstack/react-router";

import { loadStartPage } from "./start-page-setting.ts";
import { paths } from "./paths.ts";
import { forgetDeviceChoices } from "./personal-sources.ts";
import { forgetPerson } from "./session.ts";

/** What the server answers when somebody signed in or made an account. */
export type SignedIn = Schema<"SignedInBody">;

/**
 * What to do with the answer of a sign-in, whichever way it came: a password, a passkey, an
 * invitation, signing up or the setup. A person whose account asks for a second step goes to
 * that page first; everybody else goes to their start page (CFG-04).
 */
export function useAfterSignIn(): (answer: SignedIn) => void {
  const navigate = useNavigate();
  const router = useRouter();
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
      // From now on the person's choices live on the server: what the device kept before is gone.
      forgetDeviceChoices();
      // The person's start page, which the server keeps with their other settings.
      void loadStartPage().then((start) => {
        router.history.push(start);
      });
    }
  };
}
