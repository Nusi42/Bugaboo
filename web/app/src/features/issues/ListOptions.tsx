import { t } from "@bugaboo/messages";
import { Button, part, partOf, useLayout } from "@bugaboo/ui";

import type { ListView } from "../../shared/list-view.ts";
import { ViewChoices, type SearchFieldsAnswer } from "../search/index.ts";

export interface ListOptionsProps {
  readonly view: ListView;
  readonly fields: SearchFieldsAnswer;
  readonly customizing: boolean;
  readonly onView: (view: ListView) => void;
  readonly onCustomize: () => void;
}

/**
 * What a person sets above the list on a computer (UX section 4.2): *Group by* any field they may
 * read, *Spacing*, and *Customize* for the columns. A phone has none of it here: it shows a card for
 * each issue, and *Group by* and *Spacing* are in its *Filters* sheet (M5 exit decision D2).
 */
export function ListOptions({ view, fields, customizing, onView, onCustomize }: ListOptionsProps) {
  if (useLayout() === "phone") {
    return null;
  }
  return (
    <div {...partOf(part.listOptions)}>
      <ViewChoices
        view={view}
        fields={fields}
        onView={onView}
        end={
          <Button current={customizing} onPress={onCustomize}>
            {t("issues.customize")}
          </Button>
        }
      />
    </div>
  );
}
