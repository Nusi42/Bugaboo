import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Checkbox, partOf, part } from "@bugaboo/ui";

import { permissionSentence, topicHeading } from "../../shared/access-words.ts";
import { useAskFirst } from "../../shared/ask-first.tsx";
import type { PermissionKey } from "../../shared/permissions.ts";

export interface RolePermissionsProps {
  /** The ID of the role. */
  readonly role: string;
  /** The catalogue, topic by topic. */
  readonly catalogue: readonly Schema<"PermissionBody">[];
  /** What the role allows now. */
  readonly held: readonly PermissionKey[];
  /** Called with everything the role should allow from now on. */
  readonly onChange: (permissions: readonly PermissionKey[]) => void;
}

/** The permissions of one topic, in the order of the catalogue. */
interface TopicGroup {
  readonly topic: Schema<"Topic">;
  readonly permissions: readonly Schema<"PermissionBody">[];
}

/** The catalogue as groups: the permissions of a topic stand together there. */
function groupsOf(catalogue: readonly Schema<"PermissionBody">[]): readonly TopicGroup[] {
  const groups: { topic: Schema<"Topic">; permissions: Schema<"PermissionBody">[] }[] = [];
  for (const permission of catalogue) {
    const last = groups.at(-1);
    if (last?.topic === permission.topic) {
      last.permissions.push(permission);
    } else {
      groups.push({ topic: permission.topic, permissions: [permission] });
    }
  }
  return groups;
}

/**
 * The permissions of a role grouped by topic, each a checkbox with its plain sentence (UX §5.2). A
 * permission that counts only for the whole organization says so. Ticking or unticking saves at once;
 * taking one away asks first when it would take away the person's own way to manage roles, because
 * they may hold the role themselves.
 */
export function RolePermissions({ role, catalogue, held, onChange }: RolePermissionsProps) {
  const { ask, dialog } = useAskFirst();
  const toggle = (permission: PermissionKey, on: boolean): void => {
    if (on) {
      onChange([...held, permission]);
      return;
    }
    const permissions = held.filter((known) => known !== permission);
    void ask({ kind: "change-role", role, permissions }).then((go) => {
      if (go) {
        onChange(permissions);
      }
    });
  };
  return (
    <div {...partOf(part.stack)}>
      {groupsOf(catalogue).map((group) => (
        <fieldset key={group.topic} {...partOf(part.rolesTopic)}>
          <legend {...partOf(part.rolesLegend)}>{t(topicHeading(group.topic))}</legend>
          {group.permissions.map((permission) => (
            <div key={permission.key} {...partOf(part.rolesPermission)}>
              <Checkbox
                label={t(permissionSentence(permission.key))}
                isSelected={held.includes(permission.key)}
                onChange={(on) => {
                  toggle(permission.key, on);
                }}
              />
              {permission.scope === "organization" ? (
                <span {...partOf(part.rolesNote)}>{t("roles.counts-organization")}</span>
              ) : null}
            </div>
          ))}
        </fieldset>
      ))}
      {dialog}
    </div>
  );
}
