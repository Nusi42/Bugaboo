import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";

import { heldText } from "../../shared/access-words.ts";
import { formatDateTime } from "../../shared/format.ts";
import { groupAddress, paths } from "../../shared/paths.ts";
import type { PersonLine } from "./people.ts";
import styles from "./People.module.css";

/** The groups of the person, for a viewer who may see groups. */
export function Groups({ line }: { readonly line: PersonLine }) {
  if (line.groups === null) {
    return null;
  }
  return (
    <Card heading={t("person.groups")}>
      {line.groups.length === 0 ? (
        <p className={styles.muted}>{t("person.groups-none", { name: line.name })}</p>
      ) : (
        <ul className={styles.list} aria-label={t("person.groups")}>
          {line.groups.map((group) => (
            <li key={group.id} className={styles.item}>
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
        <p className={styles.muted}>{t("person.roles-none", { name: line.name })}</p>
      ) : (
        <ul className={styles.list} aria-label={t("person.roles")}>
          {line.roles.map((held) => (
            <li key={held.id} className={styles.item}>
              <span className={styles.itemTitle}>{heldText(held)}</span>
            </li>
          ))}
        </ul>
      )}
      <p className={styles.muted}>{t("person.roles-where")}</p>
    </Card>
  );
}

/** The address and the day the account was made. */
export function Facts({ line, since }: { readonly line: PersonLine; readonly since: string }) {
  return (
    <Card heading={line.name} level={1}>
      <dl className={styles.facts}>
        <div className={styles.fact}>
          <dt className={styles.term}>{t("person.email")}</dt>
          <dd className={styles.definition}>{line.email}</dd>
        </div>
        <div className={styles.fact}>
          <dt className={styles.term}>{t("person.since")}</dt>
          <dd className={styles.definition}>{formatDateTime(since)}</dd>
        </div>
      </dl>
      <div className={styles.row}>
        <Link href={paths.people}>{t("person.back")}</Link>
      </div>
    </Card>
  );
}
