import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link, Switch, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { FieldSections } from "./FieldSections.tsx";
import { detailsOf, useField, type OwnField } from "./fields.ts";
import { useChangeField } from "./use-change-field.ts";

/** *Retire the field*: no new values, while the old ones stay. A switch, because it can be taken back. */
function RetireField({ field }: { readonly field: OwnField }) {
  const change = useChangeField();
  return (
    <Card heading={t("fields.retire-title")}>
      <p {...partOf(part.muted)}>{t("fields.retire-help")}</p>
      <Switch
        label={field.retired ? t("fields.offer-again") : t("fields.retire")}
        isSelected={field.retired}
        onChange={(retired) => {
          change.mutate({
            params: { path: { field: field.id } },
            body: { version: field.version, retired },
          });
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </Card>
  );
}

/** The page of a field: the sections of UX §5.3, one under the other. */
function Page({ field }: { readonly field: OwnField }) {
  usePageTitle(field.name);
  return (
    <div {...partOf(part.stack)}>
      <Card heading={field.name} level={1} back={{ href: paths.fields, label: t("fields.back") }}>
        <p {...partOf(part.muted)}>{detailsOf(field)}</p>
      </Card>
      <FieldSections field={field} />
      <Card heading={t("fields.see-title")}>
        <p {...partOf(part.text)}>{t("fields.see-help")}</p>
        <Link href={paths.fieldPermissions}>{t("fields.see-link")}</Link>
      </Card>
      <RetireField field={field} />
    </div>
  );
}

/** *Organization settings › Fields › one field* (CF-01 to CF-07). */
export function FieldPage({ field }: { readonly field: string }) {
  const found = useField(field);
  return (
    <QueryGate
      error={found.error}
      data={found.data}
      notFound={<p {...partOf(part.muted)}>{t("fields.not-found")}</p>}
    >
      {(known) => <Page key={known.version} field={known} />}
    </QueryGate>
  );
}
