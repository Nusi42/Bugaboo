import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { PermissionsCard } from "./PermissionsCard.tsx";
import { RoleActions } from "./RoleActions.tsx";
import { RoleName } from "./RoleName.tsx";
import styles from "./Roles.module.css";
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
        <p className={styles.muted}>
          {t(own ? "roles.own" : "roles.starting-badge")} {roleUsedText(used)}
        </p>
        <RoleName key={role.name} id={role.id} name={role.name} />
        <div className={styles.row}>
          <Link href={paths.roles}>{t("roles.back")}</Link>
        </div>
      </Card>
      <PermissionsCard role={role} />
      <RoleActions role={role} used={used} />
    </>
  );
}
