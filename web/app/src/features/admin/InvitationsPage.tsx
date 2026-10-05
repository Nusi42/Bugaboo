import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Admin.module.css";
import { CreateInvitation } from "./CreateInvitation.tsx";
import { OpenInvitations } from "./OpenInvitations.tsx";
import { SignUpPolicySection } from "./SignUpPolicySection.tsx";

/** The invitations page: make an invitation, see the open ones, and decide who may sign up alone. */
export function InvitationsPage() {
  usePageTitle(t("invitations.title"));
  return (
    <div className={styles.stack}>
      <Card heading={t("invitations.title")} level={1}>
        <p className={styles.text}>{t("invitations.intro")}</p>
        <CreateInvitation />
      </Card>
      <Card heading={t("invitations.open")}>
        <OpenInvitations />
      </Card>
      <SignUpPolicySection />
    </div>
  );
}
