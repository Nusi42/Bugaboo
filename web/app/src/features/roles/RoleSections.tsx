import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, PageBack, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { PermissionsCard } from "./PermissionsCard.tsx";
import { RoleActions } from "./RoleActions.tsx";
import { RoleName } from "./RoleName.tsx";
import { roleUsedText } from "./used-by-text.ts";

export interface RoleSectionsProps {
  readonly role: Schema<"RoleBody">;
  readonly used: Schema<"RoleUsedByBody">;
}

/**
 * The cards of the page of a role: its name and what it is used for, the switch and the permissions,
 * and what can be done with the whole role (reset a starting role, copy it, delete it).
 */
export function RoleSections({ role, used }: RoleSectionsProps) {
  const own = role.starting === undefined || role.starting === null;
  return (
    <>
      <Card heading={role.name} level={1}>
        <p {...partOf(part.muted)}>
          {t("roles.badge-used", {
            badge: t(own ? "roles.own" : "roles.starting-badge"),
            used: roleUsedText(used),
          })}
        </p>
        <RoleName key={role.name} id={role.id} name={role.name} version={role.version} />
        <PageBack href={paths.roles} label={t("roles.back")} />
      </Card>
      <PermissionsCard role={role} />
      <RoleActions role={role} used={used} />
    </>
  );
}
