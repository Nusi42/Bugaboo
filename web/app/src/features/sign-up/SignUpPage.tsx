import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { CheckYourEmail } from "./CheckYourEmail.tsx";
import { SignUpForm } from "./SignUpForm.tsx";

/**
 * Ask for an account: only while an administrator allows people to sign up on their own, which
 * needs e-mail. The answer is always *Check your e-mail*, the same for every address.
 */
export function SignUpPage() {
  usePageTitle(t("sign-up.title"));
  const open = useQuery(statusOptions()).data?.sign_up_open;
  const [sentTo, setSentTo] = useState<string>();
  if (sentTo !== undefined) {
    return (
      <CheckYourEmail
        email={sentTo}
        onOtherAddress={() => {
          setSentTo(undefined);
        }}
      />
    );
  }
  return (
    <Card heading={t("sign-up.title")} level={1}>
      {open === false ? (
        <Alert tone={alertTone.warning}>{t("sign-up.closed")}</Alert>
      ) : (
        <SignUpForm onSent={setSentTo} />
      )}
      <Link href={paths.signIn}>{t("sign-up.have-account")}</Link>
    </Card>
  );
}
