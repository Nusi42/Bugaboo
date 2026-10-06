import { createRoute, redirect } from "@tanstack/react-router";

import {
  isSignInReason,
  SecondStepPage,
  SetUpSecondStepPage,
  SignInPage,
  type SignInReason,
} from "../features/sign-in/index.ts";
import { paths } from "../shared/paths.ts";
import { guestRoute, setUpRoute } from "./layouts.tsx";
import { startPath } from "./start.ts";

export const signInRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.signIn,
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { reason?: SignInReason | undefined } => ({
    reason: isSignInReason(search["reason"]) ? search["reason"] : undefined,
  }),
  beforeLoad: async (): Promise<void> => {
    // While nobody has an account, signing in cannot work: the setup comes first.
    if ((await startPath()) === paths.setup) {
      throw redirect({ to: paths.setup });
    }
  },
  component: SignInPage,
});

/** The address is `?app=true&passkey=true`: which ways there are, for a reload to know. */
function flag(value: unknown): boolean {
  return value === true || value === "true";
}

export const secondStepRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.secondStep,
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { app: boolean; passkey: boolean } => ({
    app: flag(search["app"]),
    passkey: flag(search["passkey"]),
  }),
  component: SecondStepPage,
});

/** *Set up your second step*, for a person whose role or organization asks for one. */
export const setUpSecondStepRoute = createRoute({
  getParentRoute: () => setUpRoute,
  path: paths.setUpSecondStep,
  component: SetUpSecondStepPage,
});
