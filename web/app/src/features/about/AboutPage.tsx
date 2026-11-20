import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, partOf, part, useHoldWhileReading } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { downloadKinds, downloadMessages } from "./about.ts";
import { AboutFacts } from "./AboutFacts.tsx";
import { PartsList } from "./PartsList.tsx";

type About = Schema<"AboutBody">;

/** The parts list in both formats and the third-party notices, each saved by the browser. */
function Downloads({ about }: { readonly about: About }) {
  return (
    <div {...partOf(part.row)}>
      {downloadKinds(about.files).map((file) => (
        <Button
          key={file.kind}
          onPress={() => {
            window.location.assign(file.address);
          }}
        >
          {t(downloadMessages[file.kind])}
        </Button>
      ))}
    </div>
  );
}

/** *About* (COMP-01, COMP-04): the version, the support period, the license, and the parts inside, for everybody who is signed in. */
export function AboutPage() {
  usePageTitle(t("nav.about"));
  const about = useQuery(api.query.queryOptions("get", "/api/v1/about"));
  useHoldWhileReading(about.isLoading);
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.about")} level={1} intro={t("about.intro")}>
        {about.isError ? <Alert tone={alertTone.danger}>{messageOf(about.error)}</Alert> : null}
        {about.data === undefined ? null : <AboutFacts about={about.data} />}
      </Card>
      <Card
        heading={t("about.parts")}
        intro={
          about.data === undefined
            ? undefined
            : t("about.parts-count", { count: about.data.part_count })
        }
      >
        {about.data === undefined ? null : <Downloads about={about.data} />}
        {about.data === undefined ? null : <PartsList />}
      </Card>
    </div>
  );
}
