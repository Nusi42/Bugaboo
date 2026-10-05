import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useSearch } from "@tanstack/react-router";

import { usePageTitle } from "../../shared/use-page.ts";
import { PasskeySignIn } from "./PasskeySignIn.tsx";
import { PasswordForm } from "./PasswordForm.tsx";
import { ReasonNotice } from "./ReasonNotice.tsx";
import { SignUpLink } from "./SignUpLink.tsx";

/** Sign in: with a passkey, or with e-mail address and password. */
export function SignInPage() {
  const { reason } = useSearch({ strict: false });
  usePageTitle(t("sign-in.title"));
  return (
    <Card heading={t("sign-in.title")} level={1}>
      {reason === undefined ? null : <ReasonNotice reason={reason} />}
      <PasskeySignIn />
      <PasswordForm />
      <SignUpLink />
    </Card>
  );
}
