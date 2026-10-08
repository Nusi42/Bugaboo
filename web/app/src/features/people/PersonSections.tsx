import { t } from "@bugaboo/messages";
import { Card, Link, partOf, part } from "@bugaboo/ui";

import { heldText } from "../../shared/access-words.ts";
import { formatDateTime } from "../../shared/format.ts";
import { groupAddress, paths } from "../../shared/paths.ts";
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
            <li key={group.id} {...partOf(part.peopleItem)}>
              <Link href={groupAddress(group.id)}>{group.name}</Link>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

/** The rows of *Who has which role* of the person, in the places where the viewer decides. */
export function Roles({ line }: { readonly line: PersonLine }) {
  return (
    <Card heading={t("person.roles")}>
      {line.roles.length === 0 ? (
        <p {...partOf(part.muted)}>{t("person.roles-none", { name: line.name })}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("person.roles")}>
          {line.roles.map((held) => (
            <li key={held.id} {...partOf(part.peopleItem)}>
              <span {...partOf(part.peopleItemTitle)}>{heldText(held)}</span>
            </li>
          ))}
        </ul>
      )}
      <p {...partOf(part.muted)}>{t("person.roles-where")}</p>
    </Card>
  );
}

/** The address and the day the account was made. */
export function Facts({ line, since }: { readonly line: PersonLine; readonly since: string }) {
  return (
    <Card heading={line.name} level={1}>
      <dl {...partOf(part.peopleFacts)}>
        <div {...partOf(part.peopleFact)}>
          <dt {...partOf(part.peopleTerm)}>{t("person.email")}</dt>
          <dd {...partOf(part.peopleDefinition)}>{line.email}</dd>
        </div>
        <div {...partOf(part.peopleFact)}>
          <dt {...partOf(part.peopleTerm)}>{t("person.since")}</dt>
          <dd {...partOf(part.peopleDefinition)}>{formatDateTime(since)}</dd>
        </div>
      </dl>
      <div {...partOf(part.row)}>
        <Link href={paths.people}>{t("person.back")}</Link>
      </div>
    </Card>
  );
}
