import { t } from "@bugaboo/messages";
import { Button, ChoiceGroup, SelectField, part, partOf, useLayout } from "@bugaboo/ui";

import { noGrouping, type ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import type { SearchSpacing } from "../../shared/search-address.ts";

/** The two spacings, by the words the address and the saved search use. */
const spacings: readonly SearchSpacing[] = ["comfortable", "compact"];

/** A field the list can be grouped by. */
export interface GroupChoice {
  readonly word: string;
  readonly name: string;
}

/** The words that cannot be grouped by: a long text and the Issue ID have no groups. */
const notGroupable: ReadonlySet<string> = new Set(["description", "id"]);

export interface ListOptionsProps {
  readonly view: ListView;
  readonly fields: SearchFieldsAnswer;
  readonly customizing: boolean;
  readonly onView: (view: ListView) => void;
  readonly onCustomize: () => void;
}

/** The fields a list can be grouped by: every one the person may read. */
function groupChoicesOf(fields: SearchFieldsAnswer) {
  return Object.keys(fields.names)
    .filter((word) => !notGroupable.has(word))
    .map((word) => ({ value: word, label: fields.names[word] ?? word }));
}

/**
 * What a person sets above the list (UX section 4.2): *Group by* any field they may read,
 * *Spacing*, and *Customize* for the columns, which a phone does not offer (it shows a card for
 * each issue).
 */
export function ListOptions({ view, fields, customizing, onView, onCustomize }: ListOptionsProps) {
  const phone = useLayout() === "phone";
  return (
    <div {...partOf(part.listOptions)}>
      <SelectField
        label={t("issues.group-by")}
        choices={[
          { value: noGrouping, label: t("issues.group-by-none") },
          ...groupChoicesOf(fields),
        ]}
        value={view.groupBy ?? noGrouping}
        onChange={(value) => {
          onView({ ...view, groupBy: value === noGrouping ? null : value });
        }}
      />
      <ChoiceGroup
        legend={t("appearance.spacing.legend")}
        choices={spacings.map((word) => ({ value: word, label: t(`appearance.spacing.${word}`) }))}
        value={view.spacing}
        onChange={(spacing) => {
          onView({ ...view, spacing });
        }}
      />
      {phone ? null : (
        <Button current={customizing} onPress={onCustomize}>
          {t("issues.customize")}
        </Button>
      )}
    </div>
  );
}
