import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Icon,
  iconName,
  iconSize,
  part,
  partOf,
  selectLook,
  useLayout,
} from "@bugaboo/ui";

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
 * read, and *Customize* for the columns, as quiet words on one line that do not compete with the
 * list. A phone has none of it here: it shows a card for each
 * issue, and *Group by* is in its *Filters* sheet (M5 exit decision D2). How close together the
 * rows are is the person's *Spacing* in *My settings › Appearance* (ADR-0034, amendment).
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
        look={selectLook.inline}
        end={
          <Button look={buttonLook.quiet} current={customizing} onPress={onCustomize}>
            <Icon name={iconName.fields} size={iconSize.small} />
            {t("issues.customize")}
          </Button>
        }
      />
    </div>
  );
}
