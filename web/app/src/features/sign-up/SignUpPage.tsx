import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { SignUpForm } from "./SignUpForm.tsx";

/** Make an account: only while an administrator allows people to sign up on their own. */
export function SignUpPage() {
  usePageTitle(t("sign-up.title"));
  const open = useQuery(statusOptions()).data?.sign_up_open;
  return (
    <Card heading={t("sign-up.title")} level={1}>
      {open === false ? (
        <Alert tone={alertTone.warning}>{t("sign-up.closed")}</Alert>
      ) : (
        <SignUpForm />
      )}
      <Link href={paths.signIn}>{t("sign-up.have-account")}</Link>
    </Card>
  );
}
