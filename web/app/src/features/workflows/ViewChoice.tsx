import { t } from "@bugaboo/messages";
import { ChoiceGroup } from "@bugaboo/ui";

import type { View, ViewWord } from "./use-view.ts";

/** *List* and *Drawing*, on a computer; nothing on a phone, which has the list only. */
export function ViewChoice({ view }: { readonly view: View }) {
  if (!view.canDraw) {
    return null;
  }
  return (
    <ChoiceGroup<ViewWord>
      legend={t("workflows.view")}
      choices={[
        { value: "list", label: t("workflows.view-list") },
        { value: "drawing", label: t("workflows.view-drawing") },
      ]}
      value={view.drawing ? "drawing" : "list"}
      onChange={view.choose}
    />
  );
}
