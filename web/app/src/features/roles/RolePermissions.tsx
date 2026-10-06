import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, Button, Checkbox } from "@bugaboo/ui";
import { useState } from "react";

import { permissionSentence, topicHeading } from "../../shared/access-words.ts";
import styles from "./Roles.module.css";
import type { PermissionKey } from "./roles.ts";

/** The permissions that let somebody manage roles: taking one away asks first. */
const MANAGING: ReadonlySet<PermissionKey> = new Set(["manage-roles", "decide-roles"]);

export interface RolePermissionsProps {
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

/** The question that comes before taking away a permission that manages roles. */
function AskFirst({
  onKeep,
  onTakeAway,
}: {
  readonly onKeep: () => void;
  readonly onTakeAway: () => void;
}) {
  return (
    <Alert warning>
      <p className={styles.text}>{t("roles.ask-first")}</p>
      <div className={styles.row}>
        <Button onPress={onKeep}>{t("roles.ask-first-keep")}</Button>
        <Button danger onPress={onTakeAway}>
          {t("roles.ask-first-yes")}
        </Button>
      </div>
    </Alert>
  );
}

/**
 * The permissions of a role grouped by topic, each a checkbox with its plain sentence (UX §5.2). A
 * permission that counts only for the whole organization says so. Ticking or unticking saves at once;
 * taking away a permission that manages roles asks first, because the person who does it may hold the
 * role themselves.
 */
export function RolePermissions({ catalogue, held, onChange }: RolePermissionsProps) {
  const [asking, setAsking] = useState<PermissionKey | null>(null);
  const toggle = (permission: PermissionKey, on: boolean): void => {
    if (!on && MANAGING.has(permission)) {
      setAsking(permission);
      return;
    }
    onChange(on ? [...held, permission] : held.filter((known) => known !== permission));
  };
  return (
    <div className={styles.stack}>
      {groupsOf(catalogue).map((group) => (
        <fieldset key={group.topic} className={styles.topic}>
          <legend className={styles.legend}>{t(topicHeading(group.topic))}</legend>
          {group.permissions.map((permission) => (
            <div key={permission.key} className={styles.permission}>
              <Checkbox
                label={t(permissionSentence(permission.key))}
                isSelected={held.includes(permission.key)}
                onChange={(on) => {
                  toggle(permission.key, on);
                }}
              />
              {permission.scope === "organization" ? (
                <span className={styles.note}>{t("roles.counts-organization")}</span>
              ) : null}
            </div>
          ))}
        </fieldset>
      ))}
      {asking === null ? null : (
        <AskFirst
          onKeep={() => {
            setAsking(null);
          }}
          onTakeAway={() => {
            onChange(held.filter((known) => known !== asking));
            setAsking(null);
          }}
        />
      )}
    </div>
  );
}
