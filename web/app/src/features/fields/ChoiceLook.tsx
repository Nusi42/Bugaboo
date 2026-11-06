import { issueIconWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { SelectField, Switch } from "@bugaboo/ui";

import { ColorChoice } from "../../shared/ColorChoice.tsx";
import type { Choice } from "./fields.ts";

/** Words of the HTML standard and marks that are not text for people. */
const noIcon = "none";

const iconChoices = [
  { value: noIcon, label: t("fields.no-icon") },
  ...issueIconWords.map((word) => ({ value: word, label: t(`issue-icon.${word}`) })),
];

/** Sends a change of the choice, on the version of the field the person looked at. */
type Send = (more: Readonly<Record<string, unknown>>) => void;

/** The color and icon of the choice, and the switch that retires it. */
export function ChoiceLook({ choice, send }: { readonly choice: Choice; readonly send: Send }) {
  return (
    <>
      <ColorChoice
        legend={t("fields.choice-color", { name: choice.shown_as })}
        value={choice.color}
        onChange={(color) => {
          send({ color });
        }}
      />
      <SelectField
        label={t("fields.choice-icon", { name: choice.shown_as })}
        choices={iconChoices}
        value={choice.icon ?? noIcon}
        onChange={(icon) => {
          send({ icon: icon === noIcon ? null : icon });
        }}
      />
      <Switch
        label={choice.retired ? t("fields.choice-offer-again") : t("fields.choice-retire")}
        isSelected={choice.retired}
        onChange={(retired) => {
          send({ retired });
        }}
      />
    </>
  );
}
