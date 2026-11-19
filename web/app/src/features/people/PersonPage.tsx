import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { paths } from "../../shared/paths.ts";
import { NotFoundPage, QueryGate, useItemTrail } from "../shell/index.ts";
import { lineOf, usePerson } from "./people.ts";
import { Corrections } from "./PersonCorrections.tsx";
import { Facts, Groups, HeldRoles, WorkTimesEntry } from "./PersonSections.tsx";

/**
 * The page of one person (SEC-02): their address, when they came, the groups they are in and the
 * roles they have, as far as the viewer may see them. Changing them is done where the rows are:
 * in *Groups* and in *Who has which role*. The name and the address of somebody else are corrected
 * here (PRIV-06); one's own are changed in *My settings › Profile*.
 */
export function PersonPage({ id }: { readonly id: string }) {
  const found = usePerson(id);
  const own = useMe()?.id === id;
  const line = found.data === undefined ? undefined : lineOf(found.data.person);
  const shown =
    found.data !== undefined && line !== undefined
      ? { line, since: found.data.created_at }
      : undefined;
  usePageTitle(line?.name ?? t("nav.people"));
  useItemTrail(paths.people, line?.name);
  return (
    <QueryGate error={found.error} data={shown} notFound={<NotFoundPage />}>
      {(person) => (
        <div {...partOf(part.stack)}>
          <Facts line={person.line} since={person.since} />
          <Groups line={person.line} />
          <HeldRoles
            roles={person.line.roles}
            none={t("person.roles-none", { name: person.line.name })}
          />
          <WorkTimesEntry id={id} name={person.line.name} />
          {own ? null : <Corrections id={id} name={person.line.name} email={person.line.email} />}
        </div>
      )}
    </QueryGate>
  );
}
