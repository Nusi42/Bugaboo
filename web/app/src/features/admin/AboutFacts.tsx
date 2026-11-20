import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatCalendarDay } from "../../shared/format.ts";
import { supportOf } from "./about.ts";

type About = Schema<"AboutBody">;

/** One line of the facts: what it is, and what it says. */
function Fact({ term, children }: { readonly term: string; readonly children: ReactNode }) {
  return (
    <div {...partOf(part.fact)}>
      <dt {...partOf(part.term)}>{term}</dt>
      <dd {...partOf(part.adminDetail)}>{children}</dd>
    </div>
  );
}

/** How long this release gets security fixes, in words. */
function supportText(about: About): string {
  const support = supportOf(about.support_period);
  return support.kind === "until"
    ? t("about.support-until", { date: formatCalendarDay(support.day) })
    : t("about.support-preview");
}

/** The facts of the release: its version, its day, its support period and its license. */
export function AboutFacts({ about }: { readonly about: About }) {
  return (
    <dl {...partOf(part.facts)}>
      <Fact term={t("about.version")}>{about.version}</Fact>
      <Fact term={t("about.released")}>{formatCalendarDay(about.support_period.released)}</Fact>
      <Fact term={t("about.support")}>{supportText(about)}</Fact>
      <Fact term={t("about.license")}>{about.license}</Fact>
    </dl>
  );
}
