import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotFoundPage } from "../shell/index.ts";
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
  usePageTitle(line?.name ?? t("nav.people"));
  usePersonTrail(line?.name);
  if (found.isError) {
    return codeOf(found.error) === "resource.not_found" ? (
      <NotFoundPage />
    ) : (
      <Alert danger>{messageOf(found.error)}</Alert>
    );
  }
  if (found.data === undefined || line === undefined) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  return (
    <div className={styles.stack}>
      <Facts line={line} since={found.data.created_at} />
      <Groups line={line} />
      <Roles line={line} />
    </div>
  );
}
