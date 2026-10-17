import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useLocation } from "@tanstack/react-router";

import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { AcceptForm } from "./AcceptForm.tsx";
import { useInvitation } from "./use-invitation.ts";

/**
 * The way out of a link that does not work: sign in, and, where people may sign up on their own, ask
 * for a new link (a link of a sign-up works once and for a day).
 */
function WayOut() {
  const signUpOpen = useQuery(statusOptions()).data?.sign_up_open === true;
  return (
    <>
      {signUpOpen ? <Link href={paths.signUp}>{t("invitation.ask-again")}</Link> : null}
      <Link href={paths.signIn}>{t("invitation.sign-in")}</Link>
    </>
  );
}

/** What the invitation link leads to: a form, or the reason it does not work. */
function Invitation({ secret }: { readonly secret: string }) {
  const invitation = useInvitation(secret);
  if (invitation.isError) {
    return (
      <Card heading={t("invitation.invalid-title")} level={1}>
        <Alert tone={alertTone.danger}>{messageOf(invitation.error)}</Alert>
        <WayOut />
      </Card>
    );
  }
  return (
    <Card heading={t("invitation.title")} level={1}>
      {invitation.data === undefined ? null : (
        <AcceptForm secret={secret} email={invitation.data.email} />
      )}
    </Card>
  );
}

/** The page an invitation link opens. The secret is in the part of the address after the hash sign. */
export function InvitationPage() {
  usePageTitle(t("invitation.title"));
  const secret = useLocation().hash.replace(/^#/u, "");
  if (secret === "") {
    return (
      <Card heading={t("invitation.invalid-title")} level={1}>
        <Alert tone={alertTone.warning}>{t("invitation.incomplete")}</Alert>
        <WayOut />
      </Card>
    );
  }
  return <Invitation secret={secret} />;
}
