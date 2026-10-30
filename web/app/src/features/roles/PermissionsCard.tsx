import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, SaveStatus } from "@bugaboo/ui";

import { RolePermissions } from "./RolePermissions.tsx";
import { useCatalogue } from "./roles.ts";
import { RoleSwitch } from "./RoleSwitch.tsx";
import { useRoleEdits } from "./use-role-edits.ts";
import { saveLabels } from "../../shared/save-labels.ts";

/**
 * What the role allows (UX §5.2): the switch for two-step sign-in and the permissions as checkboxes,
 * each saved at once, with *Saving…* and *Saved* next to them.
 */
export function PermissionsCard({ role }: { readonly role: Schema<"RoleBody"> }) {
  const catalogue = useCatalogue();
  const edits = useRoleEdits(role.id);
  return (
    <Card heading={t("roles.permissions-title")} intro={t("roles.permissions-intro")}>
      <SaveStatus state={edits.saveState} labels={saveLabels()} />
      {edits.problem === null ? null : <Alert tone={alertTone.danger}>{edits.problem}</Alert>}
      <RoleSwitch
        id={role.id}
        on={role.two_step}
        onChange={(on) => {
          edits.change({ kind: "two-step", on, version: role.version });
        }}
      />
      {catalogue.data === undefined ? null : (
        <RolePermissions
          role={role.id}
          catalogue={catalogue.data.permissions}
          held={role.permissions}
          onChange={(permissions) => {
            edits.change({ kind: "permissions", permissions, version: role.version });
          }}
        />
      )}
    </Card>
  );
}
