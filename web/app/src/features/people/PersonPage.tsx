import { t } from "@bugaboo/messages";

import { usePageTitle } from "../../shared/use-page.ts";
import { NotFoundPage, QueryGate } from "../shell/index.ts";
import { lineOf, usePerson } from "./people.ts";
import styles from "./People.module.css";
import { usePersonTrail } from "./person-trail.ts";
import { Facts, Groups, Roles } from "./PersonSections.tsx";

/**
 * The page of one person (SEC-02): their address, when they came, the groups they are in and the
 * roles they have, as far as the viewer may see them. Changing them is done where the rows are:
 * in *Groups* and in *Who has which role*.
 */
export function PersonPage({ id }: { readonly id: string }) {
  const found = usePerson(id);
  const line = found.data === undefined ? undefined : lineOf(found.data.person);
  const shown =
    found.data !== undefined && line !== undefined
      ? { line, since: found.data.created_at }
      : undefined;
  usePageTitle(line?.name ?? t("nav.people"));
  usePersonTrail(line?.name);
  return (
    <QueryGate error={found.error} data={shown} notFound={<NotFoundPage />}>
      {(person) => (
        <div className={styles.stack}>
          <Facts line={person.line} since={person.since} />
          <Groups line={person.line} />
          <Roles line={person.line} />
        </div>
      )}
    </QueryGate>
  );
}
