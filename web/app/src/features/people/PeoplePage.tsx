import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { CreateInvitation } from "./CreateInvitation.tsx";
import { OpenInvitations } from "./OpenInvitations.tsx";
import { PeopleList } from "./PeopleList.tsx";
import { SignUpPolicySection } from "./SignUpPolicySection.tsx";

/** What a person who holds *Invite people* sees: the form, and the invitations that are still open. */
function Invitations() {
  return (
    <>
      <Card heading={t("invitations.title")}>
        <p {...partOf(part.text)}>{t("invitations.intro")}</p>
        <CreateInvitation />
      </Card>
      <Card heading={t("invitations.open")}>
        <OpenInvitations />
      </Card>
    </>
  );
}

/**
 * *Organization settings › People* (SEC-02, AUTH-06): everybody who has an account, for those who
 * manage people; the invitations (they moved here from their own page) for those who may invite;
 * and the rule for who may sign up alone, for those who change organization settings.
 */
export function PeoplePage() {
  usePageTitle(t("nav.people"));
  const permissions = usePermissions();
  const mayList = permissions.includes("manage-people");
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.people")} level={1}>
        <p {...partOf(part.text)}>{t("people.intro")}</p>
        {mayList ? <PeopleList /> : <p {...partOf(part.muted)}>{t("people.list-ask")}</p>}
      </Card>
      {permissions.includes("manage-invitations") ? <Invitations /> : null}
      {permissions.includes("change-organization-settings") ? <SignUpPolicySection /> : null}
    </div>
  );
}
