import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { CopyRole } from "./CopyRole.tsx";
import { DeleteRole } from "./DeleteRole.tsx";
import { ResetRole } from "./ResetRole.tsx";
import styles from "./Roles.module.css";

export interface RoleActionsProps {
  readonly role: Schema<"RoleBody">;
  readonly used: Schema<"RoleUsedByBody">;
}

/** What can be done with the whole role: reset a starting role, copy it, delete it. */
export function RoleActions({ role, used }: RoleActionsProps) {
  const starting = role.starting !== undefined && role.starting !== null;
  return (
    <Card heading={t("roles.actions-title")}>
      <div className={styles.stack}>
        {starting ? <ResetRole id={role.id} name={role.name} /> : null}
        <CopyRole id={role.id} name={role.name} />
        <DeleteRole id={role.id} name={role.name} used={used} />
      </div>
    </Card>
  );
}
