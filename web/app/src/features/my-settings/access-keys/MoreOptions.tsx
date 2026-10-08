import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Switch, partOf, part } from "@bugaboo/ui";

import type { Draft } from "./draft.ts";
import { PermissionPicker, ProjectPicker } from "./Pickers.tsx";

export interface MoreOptionsProps {
  readonly draft: Draft;
  readonly rules: Schema<"AccessKeyRulesBody">;
  /** The person switched single permissions on or off. */
  readonly onSwitched: () => void;
}

/** *More options*: single permissions instead of the three choices, and the projects. */
export function MoreOptions({ draft, rules, onSwitched }: MoreOptionsProps) {
  return (
    <details {...partOf(part.accessKeysMore)}>
      <summary {...partOf(part.accessKeysMoreTitle)}>{t("access-keys.more")}</summary>
      <div {...partOf(part.accessKeysMoreBody)}>
        <Switch
          label={t("access-keys.single")}
          isSelected={draft.single}
          onChange={(on) => {
            draft.setSingle(on);
            onSwitched();
          }}
        />
        {draft.single ? <PermissionPicker choices={rules.permissions} draft={draft} /> : null}
        <ProjectPicker draft={draft} />
      </div>
    </details>
  );
}
