import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { paths } from "../../shared/paths.ts";
import { NotFoundPage, QueryGate, useItemTrail } from "../shell/index.ts";
import { lineOf, pageNameOf, usePerson } from "./people.ts";
import { Corrections } from "./PersonCorrections.tsx";
import { PersonState } from "./PersonState.tsx";
import {
  Facts,
  FormerPerson,
  Groups,
  HeldRoles,
  PersonHead,
  WorkTimesEntry,
} from "./PersonSections.tsx";

/**
 * The page of one person (SEC-02): their address, when they came, the groups they are in and the
 * roles they have, as far as the viewer may see them. Changing them is done where the rows are:
 * in *Groups* and in *Who has which role*. The name and the address of somebody else are corrected
 * here (PRIV-06); one's own are changed in *My settings › Profile*. Somebody else is switched off, switched on and
 * removed here too (PRIV-04), and a removed person's page shows *Former user K7Q2* in place.
 */
export function PersonPage({ id }: { readonly id: string }) {
  const found = usePerson(id);
  const own = useMe()?.id === id;
  const line = found.data === undefined ? undefined : lineOf(found.data.person);
  const person = found.data?.person.person;
  const former = person?.shown === "former" ? person.code : undefined;
  const shown =
    found.data !== undefined && (line !== undefined || former !== undefined)
      ? { line, former, since: found.data.created_at }
      : undefined;
  const title = found.data === undefined ? undefined : pageNameOf(found.data.person);
  usePageTitle(title ?? t("nav.people"));
  useItemTrail(paths.people, title);
  return (
    <QueryGate error={found.error} data={shown} notFound={<NotFoundPage />}>
      {(page) => (
        <div {...partOf(part.stack)}>
          <PersonHead name={title ?? ""}>
            {page.line === undefined ? (
              <FormerPerson code={page.former ?? ""} />
            ) : (
              <Facts line={page.line} since={page.since} />
            )}
          </PersonHead>
          {page.line === undefined ? null : (
            <>
              <Groups line={page.line} />
              <HeldRoles
                roles={page.line.roles}
                none={t("person.roles-none", { name: page.line.name })}
              />
              <WorkTimesEntry id={id} name={page.line.name} />
              {own ? null : (
                <>
                  <Corrections id={id} name={page.line.name} email={page.line.email} />
                  <PersonState id={id} name={page.line.name} standing={page.line.standing} />
                </>
              )}
            </>
          )}
        </div>
      )}
    </QueryGate>
  );
}
