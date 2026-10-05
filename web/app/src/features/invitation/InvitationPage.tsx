import { ApiError } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, Card, Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useLocation } from "@tanstack/react-router";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { AcceptForm } from "./AcceptForm.tsx";

/** Asks the server which address an invitation is for, which also shows whether it still works. */
function useInvitation(secret: string) {
  return useQuery({
    queryKey: ["invitation", secret],
    queryFn: async () => {
      const { data } = await api.client.POST("/api/v1/auth/invitation/lookup", {
        body: { secret },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    // The secret is not kept in memory longer than the page needs it.
    gcTime: 0,
  });
}

/** What the invitation link leads to: a form, or the reason it does not work. */
function Invitation({ secret }: { readonly secret: string }) {
  const invitation = useInvitation(secret);
  if (invitation.isError) {
    return (
      <Card heading={t("invitation.invalid-title")} level={1}>
        <Alert danger>{messageOf(invitation.error)}</Alert>
        <Link href={paths.signIn}>{t("invitation.sign-in")}</Link>
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
        <Alert warning>{t("invitation.incomplete")}</Alert>
        <Link href={paths.signIn}>{t("invitation.sign-in")}</Link>
      </Card>
    );
  }
  return <Invitation secret={secret} />;
}
