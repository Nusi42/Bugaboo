import { t } from "@bugaboo/messages";
import { Button, buttonLook, NameValueRow, partOf, part } from "@bugaboo/ui";

import type { OwnField } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";
import { ValueView } from "./ValueView.tsx";

/** One field of time entries: what this project sets, or what comes from above, and *Reset to default*. */
export function TimeDefaultLine({
  field,
  project,
  own,
  above,
  onCommit,
}: {
  readonly field: OwnField;
  readonly project: string;
  readonly own: unknown;
  readonly above: unknown;
  readonly onCommit: (field: OwnField, value: Sent) => void;
}) {
  const setHere = own !== undefined && own !== null;
  return (
    <NameValueRow name={field.name}>
      <div {...partOf(part.stack)}>
        <ValueEditor
          field={field}
          project={project}
          value={own}
          inRow
          onCommit={(sent) => {
            onCommit(field, sent);
          }}
        />
        {setHere ? (
          <div>
            <Button
              look={buttonLook.quiet}
              aria-label={t("setting.ui.reset-name", { name: field.name })}
              onPress={() => {
                onCommit(field, null);
              }}
            >
              {t("setting.ui.reset")}
            </Button>
          </div>
        ) : above !== undefined && above !== null ? (
          <p {...partOf(part.muted)}>
            {t("fields.time-default-from-above")} <ValueView field={field} value={above} />
          </p>
        ) : null}
      </div>
    </NameValueRow>
  );
}
