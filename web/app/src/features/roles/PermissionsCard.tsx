import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, Card, SaveStatus } from "@bugaboo/ui";

import { RolePermissions } from "./RolePermissions.tsx";
import { useCatalogue } from "./roles.ts";
import styles from "./Roles.module.css";
import { RoleSwitch } from "./RoleSwitch.tsx";
import { useRoleEdits } from "./use-role-edits.ts";

/**
 * What the role allows (UX §5.2): the switch for two-step sign-in and the permissions as checkboxes,
 * each saved at once, with *Saving…* and *Saved* next to them.
 */
export function PermissionsCard({ role }: { readonly role: Schema<"RoleBody"> }) {
  const catalogue = useCatalogue();
  const edits = useRoleEdits(role.id);
  return (
    <Card heading={t("roles.permissions-title")}>
      <p className={styles.text}>{t("roles.permissions-intro")}</p>
      <SaveStatus
        state={edits.saveState}
        labels={{
          saving: t("save.saving"),
          saved: t("save.saved"),
          "not-saved": t("save.not-saved"),
        }}
      />
      {edits.problem === null ? null : <Alert danger>{edits.problem}</Alert>}
      <RoleSwitch
        id={role.id}
        on={role.two_step}
        onChange={(on) => {
          edits.change({ kind: "two-step", on });
        }}
      />
      {catalogue.data === undefined ? null : (
        <RolePermissions
          role={role.id}
          catalogue={catalogue.data.permissions}
          held={role.permissions}
          onChange={(permissions) => {
            edits.change({ kind: "permissions", permissions });
          }}
        />
      )}
    </Card>
  );
}
