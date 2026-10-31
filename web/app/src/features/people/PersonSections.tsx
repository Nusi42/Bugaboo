import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, Link, RowChevron, partOf, part } from "@bugaboo/ui";

import { heldText } from "../../shared/access-words.ts";
import { formatDateTime } from "../../shared/format.ts";
import { addressInProject, groupAddress, paths } from "../../shared/paths.ts";
import type { PersonLine } from "./people.ts";

/** The groups of the person, for a viewer who may see groups. */
export function Groups({ line }: { readonly line: PersonLine }) {
  if (line.groups === null) {
    return null;
  }
  return (
    <Card heading={t("person.groups")}>
      {line.groups.length === 0 ? (
        <p {...partOf(part.muted)}>{t("person.groups-none", { name: line.name })}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("person.groups")}>
          {line.groups.map((group) => (
            <li key={group.id} {...partOf(part.item)}>
              <span {...partOf(part.rowMain)}>
                <Link row href={groupAddress(group.id)}>
                  {group.name}
                </Link>
              </span>
              <RowChevron />
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

/**
 * Where a row of *Who has which role* is changed, so the row opens it: *Roles and permissions* for the whole
 * organization, the same list in the settings of a project for a project; nothing for a project the viewer cannot see.
 */
function changedAt(place: Schema<"PlaceRef">): string | undefined {
  if (place.kind === "organization") {
    return paths.roles;
  }
  return place.kind === "project"
    ? addressInProject(paths.projectRoles, place.short_name)
    : undefined;
}

export interface HeldRolesProps {
  /** The rows of *Who has which role* of the person or the group, in the places where the viewer decides. */
  readonly roles: readonly Schema<"RoleAtRef">[];
  /** What to say when there is none. */
  readonly none: string;
  /** What to say when the list was cut at its cap; none where it never is. */
  readonly cut?: string | undefined;
}

/**
 * *Roles* of a person or a group: each row of *Who has which role* that the viewer decides about, and where they change
 * them. The page of a person and the page of a group show it the same way.
 */
export function HeldRoles({ roles, none, cut }: HeldRolesProps) {
  return (
    <Card heading={t("person.roles")}>
      {roles.length === 0 ? (
        <p {...partOf(part.muted)}>{none}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("person.roles")}>
          {roles.map((held) => {
            const where = changedAt(held.place);
            return (
              <li key={held.id} {...partOf(part.item)}>
                {where === undefined ? (
                  <span {...partOf(part.itemTitle)}>{heldText(held)}</span>
                ) : (
                  <>
                    <span {...partOf(part.rowMain)}>
                      <Link row href={where}>
                        {heldText(held)}
                      </Link>
                    </span>
                    <RowChevron />
                  </>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {cut === undefined ? null : <p {...partOf(part.muted)}>{cut}</p>}
      <p {...partOf(part.muted)}>{t("person.roles-where")}</p>
    </Card>
  );
}

/** The address and the day the account was made. */
export function Facts({ line, since }: { readonly line: PersonLine; readonly since: string }) {
  return (
    <Card heading={line.name} level={1} back={{ href: paths.people, label: t("person.back") }}>
      <dl {...partOf(part.facts)}>
        <div {...partOf(part.fact)}>
          <dt {...partOf(part.term)}>{t("person.email")}</dt>
          <dd {...partOf(part.definition)}>{line.email}</dd>
        </div>
        <div {...partOf(part.fact)}>
          <dt {...partOf(part.term)}>{t("person.since")}</dt>
          <dd {...partOf(part.definition)}>{formatDateTime(since)}</dd>
        </div>
      </dl>
    </Card>
  );
}
