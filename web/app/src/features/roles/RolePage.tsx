import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotFoundPage } from "../shell/index.ts";
import { useRoleTrail } from "./role-trail.ts";
import { RoleSections } from "./RoleSections.tsx";
import { useRole } from "./roles.ts";
import styles from "./Roles.module.css";

/**
 * The page of one role (SEC-08, UX §5.2): its name and what it is used for, the switch for two-step
 * sign-in, the permissions as checkboxes with their sentences, and what can be done with the whole
 * role: reset it to the starting permissions, copy it, delete it.
 */
export function RolePage({ id }: { readonly id: string }) {
  const found = useRole(id);
  const role = found.data?.role;
  usePageTitle(role?.name ?? t("nav.roles"));
  useRoleTrail(role?.name);
  if (found.isError) {
    return codeOf(found.error) === "resource.not_found" ? (
      <NotFoundPage />
    ) : (
      <Alert danger>{messageOf(found.error)}</Alert>
    );
  }
  if (found.data === undefined || role === undefined) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  return (
    <div className={styles.stack}>
      <RoleSections role={role} used={found.data.used_by} />
    </div>
  );
}
