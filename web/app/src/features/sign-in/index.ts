// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/sign-in";
import { lazyRouteComponent } from "@tanstack/react-router";

export { SecondStepPage } from "./SecondStepPage.tsx";
export { isSignInReason, type SignInReason } from "./reasons.ts";
export { SignInPage } from "./SignInPage.tsx";

// Loaded when the page first opens: only a person asked to set up a second step sees it, and it
// brings the sections of the account area, so the first page stays small (coding-standards
// section 6).
export const SetUpSecondStepPage = lazyRouteComponent(
  () => import("./SetUpSecondStepPage.tsx"),
  "SetUpSecondStepPage",
);
