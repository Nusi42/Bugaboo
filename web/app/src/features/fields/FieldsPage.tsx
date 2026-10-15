import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { fieldAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { detailsOf, useFields } from "./fields.ts";
import { MakeFieldForm } from "./MakeFieldForm.tsx";

/**
 * *Organization settings › Fields* (CF-01): the own fields as a list, each a link to its page, and
 * the form that makes a field. Retired fields are listed too, so they can be offered again.
 */
export function FieldsPage() {
  usePageTitle(t("nav.fields"));
  const fields = useFields({ retired: true });
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.fields")} level={1}>
        <p {...partOf(part.text)}>{t("fields.intro")}</p>
        {fields.isError ? <Alert tone={alertTone.danger}>{messageOf(fields.error)}</Alert> : null}
        <QueryGate error={fields.error} data={fields.data}>
          {(page) =>
            page.fields.length === 0 ? (
              <p {...partOf(part.muted)}>{t("fields.nothing")}</p>
            ) : (
              <ul {...partOf(part.list)} aria-label={t("fields.list")}>
                {page.fields.map((field) => (
                  <li key={field.id} {...partOf(part.item)}>
                    <Link href={fieldAddress(field.id)}>{field.name}</Link>
                    <span {...partOf(part.muted)}>{detailsOf(field)}</span>
                  </li>
                ))}
              </ul>
            )
          }
        </QueryGate>
      </Card>
      <Card heading={t("fields.make-title")}>
        <p {...partOf(part.text)}>{t("fields.make-intro")}</p>
        <MakeFieldForm />
      </Card>
    </div>
  );
}
