import { t } from "@bugaboo/messages";
import { Button, buttonLook, Switch, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { offerStateName } from "./offer-state.ts";

export interface OfferedHereProps {
  /** What is offered or not: the name of the issue type or of the group of labels. */
  readonly name: string;
  /** What stands beside the name, like the icon or a word that says it is retired. */
  readonly beside?: ReactNode;
  /** Whether the project offers it now. */
  readonly offered: boolean;
  /** The line the project has of its own; none while it follows the project above. */
  readonly own: boolean | undefined;
  /** Whether the person may change it for this project. */
  readonly mayChange: boolean;
  readonly isPending: boolean;
  /** The person chose: offered (`true`), not offered (`false`), or the project above again (`null`). */
  readonly onChange: (offered: boolean | null) => void;
}

/**
 * One line of the screen of a project: an issue type or a group of labels, and a switch that says
 * whether the project offers it (ISS-02, TAG-02). A line the project has of its own can be taken
 * away, and then the project follows the project above it again.
 */
export function OfferedHere({
  name,
  beside,
  offered,
  own,
  mayChange,
  isPending,
  onChange,
}: OfferedHereProps) {
  return (
    <li {...partOf(part.item)}>
      <div {...partOf(part.row)}>
        {mayChange ? (
          <Switch
            label={name}
            isSelected={offered}
            onChange={(next) => {
              if (!isPending) {
                onChange(next);
              }
            }}
          />
        ) : (
          <span>
            {t("projects.offer-line", {
              name,
              state: offerStateName(offered),
            })}
          </span>
        )}
        {beside}
      </div>
      {own === undefined ? (
        <span {...partOf(part.muted)}>{t("projects.offer-follows")}</span>
      ) : mayChange ? (
        <Button
          look={buttonLook.quiet}
          aria-label={t("projects.offer-follow-button-for", { name })}
          isPending={isPending}
          onPress={() => {
            onChange(null);
          }}
        >
          {t("projects.offer-follow-button")}
        </Button>
      ) : null}
    </li>
  );
}
