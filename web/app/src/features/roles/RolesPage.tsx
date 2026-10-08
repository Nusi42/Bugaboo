import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { AssignmentsSection } from "./AssignmentsSection.tsx";
import { MakeRoleForm } from "./MakeRoleForm.tsx";
import { RoleList } from "./RoleList.tsx";

/** The roles, and the form that makes one: for people who hold *Manage roles*. */
function Roles() {
  return (
    <>
      <Card heading={t("nav.roles")} level={1}>
        <p {...partOf(part.text)}>{t("roles.intro")}</p>
        <RoleList />
      </Card>
      <Card heading={t("roles.make-title")}>
        <p {...partOf(part.text)}>{t("roles.make-intro")}</p>
        <MakeRoleForm />
      </Card>
    </>
  );
}

/**
 * *Organization settings › Roles & permissions* (SEC-02, SEC-08): the roles (each opens on its own
 * page), the form that makes a role from nothing or from a copy, and *Who has which role* for the
 * whole organization. Each part is there for the people who hold its permission.
 */
export function RolesPage() {
  usePageTitle(t("nav.roles"));
  const permissions = usePermissions();
  const mayRoles = permissions.includes("manage-roles");
  return (
    <div {...partOf(part.stack)}>
      {mayRoles ? <Roles /> : null}
      {permissions.includes("decide-roles") ? (
        <AssignmentsSection project={null} level={mayRoles ? 2 : 1} />
      ) : null}
    </div>
  );
}
