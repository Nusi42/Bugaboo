import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, Link, partOf, part } from "@bugaboo/ui";

import { useLocation } from "@tanstack/react-router";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { Frame } from "./Frame.tsx";
import { usePageTrail } from "./page-trail.ts";

/** The brand, for the pages that stand outside the layouts. */
function Brand() {
  return <span {...partOf(part.shellBrand)}>{t("app.name")}</span>;
}

/** The card that says there is nothing here, and the way home. */
function NothingHere() {
  return (
    <div {...partOf(part.shellNarrow)}>
      <Card heading={t("page.not-found.title")} level={1}>
        <p {...partOf(part.text)}>{t("page.not-found.text")}</p>
        <Link href={paths.home}>{t("page.not-found.home")}</Link>
      </Card>
    </div>
  );
}

/**
 * An issue, a project or a person the signed-in person may not see, or that is not there: said
 * inside the shell, in its card, with the top bar saying what the card says.
 */
export function NotFoundPage() {
  usePageTitle(t("page.not-found.title"));
  const { pathname } = useLocation();
  const steps = useMemo(() => [{ label: t("page.not-found.title"), href: pathname }], [pathname]);
  usePageTrail(steps);
  return <NothingHere />;
}

/** An address that no page has: outside the layouts, so with the brand and the choice of theme. */
export function UnknownAddressPage() {
  usePageTitle(t("page.not-found.title"));
  return (
    <Frame header={<Brand />}>
      <NothingHere />
    </Frame>
  );
}

/** Something broke while a page was being made: nothing the person did. */
export function ErrorPage() {
  usePageTitle(t("error.server.internal"));
  return (
    <Frame header={<Brand />}>
      <div {...partOf(part.shellNarrow)}>
        <Card heading={t("app.name")} level={1}>
          <Alert tone={alertTone.danger}>{t("error.server.internal")}</Alert>
          <Button
            look={buttonLook.primary}
            onPress={() => {
              window.location.reload();
            }}
          >
            {t("common.try-again")}
          </Button>
        </Card>
      </div>
    </Frame>
  );
}

/** What shows while a page is being got ready, if that takes a moment. */
export function PendingPage() {
  return (
    <Frame header={<Brand />}>
      <p>{t("common.loading")}</p>
    </Frame>
  );
}
