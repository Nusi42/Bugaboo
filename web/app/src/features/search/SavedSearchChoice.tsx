import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { itemsOf } from "../../shared/paged.ts";
import { savedSet, useSavedList } from "./saved-searches.ts";

export interface SavedSearchChoiceProps {
  readonly label: string;
  /** The ID of the saved search chosen; empty while none is. */
  readonly value: string;
  readonly onChange: (id: string) => void;
}

/**
 * A choice among the saved searches the person can open: their own and the ones shared with them.
 * The one picker of saved searches for a window that needs one (the standing rule of an agent).
 */
export function SavedSearchChoice({ label, value, onChange }: SavedSearchChoiceProps) {
  const mine = itemsOf(useSavedList(savedSet.mine).data?.pages);
  const shared = itemsOf(useSavedList(savedSet.shared).data?.pages);
  const choices = [{ value: "", label: t("search.choose-saved") }];
  for (const saved of [...mine, ...shared]) {
    choices.push({ value: saved.id, label: saved.name });
  }
  return <SelectField label={label} value={value} onChange={onChange} choices={choices} />;
}
