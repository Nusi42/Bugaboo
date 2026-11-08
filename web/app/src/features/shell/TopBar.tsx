import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Icon,
  iconName,
  iconSize,
  IssueId,
  type LayoutMode,
  partOf,
  part,
  ariaCurrent,
} from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Link } from "react-aria-components";

import { openOverlay } from "./overlay-state.ts";
import type { TrailStep } from "./page-trail.ts";
import { useTopBar } from "./use-top-bar.ts";

/** The words of a step: an issue ID is drawn by the part of IDs. */
function StepLabel({ step }: { readonly step: TrailStep }) {
  return step.issueId === true ? <IssueId id={step.label} /> : step.label;
}

/** Where you are, as links up: `My settings › Appearance`, or the projects above a project. */
function Trail({ steps }: { readonly steps: readonly TrailStep[] }) {
  return (
    <nav aria-label={t("shell.trail")} {...partOf(part.shellFill)}>
      <ol {...partOf(part.topBarCrumbs)}>
        {steps.map((step, index) => {
          const last = index === steps.length - 1;
          return (
            <li key={step.href} {...partOf(part.topBarCrumb)}>
              {last ? (
                <span aria-current={ariaCurrent.page} {...partOf(part.strong)}>
                  <StepLabel step={step} />
                </span>
              ) : (
                <>
                  <Link href={step.href} {...partOf(part.topBarUp)}>
                    <StepLabel step={step} />
                  </Link>
                  <Icon name={iconName.next} size={iconSize.small} />
                </>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export interface TopBarProps {
  readonly layout: LayoutMode;
  /** What stands at the end of the bar: the buttons features add (`TopBarActions`). */
  readonly children?: ReactNode;
}

/**
 * The bar at the top of the content (UX §3): where you are, and what a tablet or a phone needs
 * instead of a sidebar: the *Menu* button, *Search*, or *Back*.
 */
export function TopBar({ layout, children }: TopBarProps) {
  const { steps, hasUp, goBack } = useTopBar();
  const phone = layout === "phone";
  const last = steps.at(-1);
  return (
    <header {...partOf(part.topBar)}>
      {layout === "tablet" ? (
        <Button
          onPress={() => {
            openOverlay("menu");
          }}
        >
          <Icon name={iconName.menu} />
          {t("shell.menu-button")}
        </Button>
      ) : null}
      {phone && hasUp ? (
        <Button look={buttonLook.quiet} onPress={goBack}>
          <Icon name={iconName.back} />
          {t("shell.back")}
        </Button>
      ) : null}
      {phone ? (
        <span {...partOf(part.topBarTitle)}>
          {last === undefined ? t("app.name") : <StepLabel step={last} />}
        </span>
      ) : (
        <Trail steps={steps} />
      )}
      {layout === "tablet" ? (
        <Button
          onPress={() => {
            openOverlay("search");
          }}
        >
          <Icon name={iconName.search} />
          {t("shell.search")}
        </Button>
      ) : null}
      {children}
    </header>
  );
}
