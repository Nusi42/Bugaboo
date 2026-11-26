import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  doNothing,
  Icon,
  iconName,
  iconSize,
  IssueId,
  type LayoutMode,
  partOf,
  part,
  ariaCurrent,
} from "@bugaboo/ui";
import { useEffect, useState, type ReactNode } from "react";
import { Link } from "react-aria-components";
import { useLocation, useRouterState } from "@tanstack/react-router";

import { areaAt } from "../../shared/places.ts";
import { queryClient } from "../../shared/query-client.ts";

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
            <li key={step.href ?? step.label} {...partOf(part.topBarCrumb)}>
              {last ? (
                <span aria-current={ariaCurrent.page} {...partOf(part.strong)}>
                  <StepLabel step={step} />
                </span>
              ) : (
                <>
                  {step.href === undefined ? (
                    <span>
                      <StepLabel step={step} />
                    </span>
                  ) : (
                    <Link href={step.href} {...partOf(part.topBarUp)}>
                      <StepLabel step={step} />
                    </Link>
                  )}
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

/** Whether the router has loaded the page it goes to. */
function isIdle(state: { readonly status: string }): boolean {
  return state.status === "idle";
}

/**
 * Whether the page at `pathname` has said that it has no place in the trail: it is loaded, it set none, and no read is
 * on its way any more (a page that sets its place once its first read is in has set it by then). Once said, it holds
 * until the page changes, so a read in the background does not take the title away. The reads are looked at in a task
 * of their own after each change of them, never in the same turn: a part that is drawn again and again while it waits
 * for its code makes a change of the reads each time, and drawing the title in the same turn would never let it end.
 */
function useNoPlaceSaid(pathname: string): boolean {
  const loaded = useRouterState({ select: isIdle });
  const [said, setSaid] = useState<string | null>(null);
  useEffect(() => {
    if (!loaded || said === pathname) {
      return doNothing;
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    const look = (): void => {
      timer = undefined;
      if (queryClient.isFetching() === 0) {
        setSaid(pathname);
      }
    };
    const later = (): void => {
      timer ??= setTimeout(look, 0);
    };
    const stop = queryClient.getQueryCache().subscribe(later);
    later();
    return () => {
      stop();
      clearTimeout(timer);
    };
  }, [loaded, said, pathname]);
  return said === pathname;
}

/**
 * The name of the app as the title of a phone, once the page at `pathname` has said it has no place. A part of its own:
 * it is drawn again whenever a read starts or ends, and nothing under it reads.
 */
function AppNameOnceSaid({ pathname }: { readonly pathname: string }) {
  return useNoPlaceSaid(pathname) ? t("app.name") : null;
}

/**
 * The bar at the top of the content (UX §3): where you are, and what a tablet or a phone needs
 * instead of a sidebar: the *Menu* button, *Search*, or *Back*.
 */
export function TopBar({ layout, children }: TopBarProps) {
  const { steps, hasUp, goBack } = useTopBar();
  const phone = layout === "phone";
  const last = steps.at(-1);
  const { pathname } = useLocation();
  return (
    <header {...partOf(part.topBar, { variants: [areaAt(pathname)] })}>
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
      {/* Until the page says where it is, the title of a phone stands empty in its place, which keeps the actions at the
          end: the title comes in where it stays (UX §6.4). A page that has loaded and says nothing has the name of the
          app. */}
      {phone ? (
        <span {...partOf(part.topBarTitle)}>
          {last === undefined ? <AppNameOnceSaid pathname={pathname} /> : <StepLabel step={last} />}
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
