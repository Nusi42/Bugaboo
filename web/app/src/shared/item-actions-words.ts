import { t } from "@bugaboo/messages";

/**
 * The words of the menu of a thing that is dragged (`ItemActions`): its button, named after what the actions are about
 * ("Actions for WEB-4"), and *Cancel* of its sheet on a phone. One place, so every screen that drags words it alike.
 */
export function itemActionsWords(name: string): {
  readonly label: string;
  readonly cancelLabel: string;
} {
  return { label: t("common.actions-for", { name }), cancelLabel: t("common.cancel") };
}
