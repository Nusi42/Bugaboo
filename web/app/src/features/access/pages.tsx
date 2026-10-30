import { t } from "@bugaboo/messages";
import { Card, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { CheckAccessView } from "./CheckAccessView.tsx";
import { FieldPermissionsTable, PeopleVisibilityTable } from "./tables.tsx";

/** *Organization settings › People › Field permissions* (SEC-04, UX §5.2). */
export function FieldPermissionsPage() {
  usePageTitle(t("nav.field-permissions"));
  return (
    <Card heading={t("nav.field-permissions")} level={1} intro={t("field-permissions.intro")}>
      <FieldPermissionsTable />
    </Card>
  );
}

/** The introduction of *Who sees people*: its example of a role stays on one line. */
function PeopleVisibilityIntro() {
  const example = t("people-visibility.intro-example");
  return (
    <SentenceWithValue sentence={t("people-visibility.intro", { example })} value={example}>
      {(value) => <span {...partOf(part.unbrokenWord)}>{value}</span>}
    </SentenceWithValue>
  );
}

/** *Organization settings › People › Who sees people* (PRIV-12, UX §5.2). */
export function PeopleVisibilityPage() {
  usePageTitle(t("nav.people-visibility"));
  return (
    <Card heading={t("nav.people-visibility")} level={1} intro={<PeopleVisibilityIntro />}>
      <PeopleVisibilityTable />
    </Card>
  );
}

/** *Organization settings › People › Check access* (SEC-03, UX §5.2). */
export function CheckAccessPage() {
  usePageTitle(t("nav.check-access"));
  return (
    <Card heading={t("nav.check-access")} level={1} intro={t("check-access.intro")}>
      <CheckAccessView />
    </Card>
  );
}
