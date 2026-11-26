import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  DataCell,
  DataRow,
  DataRowHead,
  DataTable,
  Link,
  dataCellVariant,
} from "@bugaboo/ui";

import { searchHref } from "../../shared/search-address.ts";
import { releaseDateText, releaseNameText, releasedOnText } from "./release-words.ts";
import type { Release, ReleaseCount } from "./releases.ts";

export interface ReleaseTableProps {
  /** The name of the table: its state. */
  readonly label: string;
  /** The short name of the project whose page it is: where a count opens its issues. */
  readonly project: string;
  readonly releases: readonly Release[];
  /** How many issues are planned for a release, by its ID. */
  readonly plannedFor: (release: string) => ReleaseCount | undefined;
  /** Opens the window of a release. */
  readonly onChange: (release: Release) => void;
}

/** The number of issues planned for a release, as a link to them as a search; plain while it names no filter. */
function Issues({
  count,
  project,
}: {
  readonly count: ReleaseCount | undefined;
  readonly project: string;
}) {
  const issues = count?.issues ?? 0;
  if (count?.filter === undefined || issues === 0) {
    return <span>{issues}</span>;
  }
  return (
    <Link
      href={searchHref({ text: count.filter, project })}
      label={t("releases.issues-link", { count: issues })}
    >
      {issues}
    </Link>
  );
}

/**
 * The releases of one state as a table (UX §4.16): the name, with where it comes from when it is made
 * above this project, its *Release date*, *Released on* and the number of issues planned for it that
 * the reader sees, which opens them as a search. *Change* stands on the releases the person may manage.
 */
export function ReleaseTable({
  label,
  project,
  releases,
  plannedFor,
  onChange,
}: ReleaseTableProps) {
  return (
    <DataTable
      label={label}
      columns={[
        t("releases.name"),
        t("releases.release-date"),
        t("releases.released-on"),
        t("releases.issues"),
        { hidden: t("releases.actions") },
      ]}
    >
      {releases.map((release) => (
        <DataRow key={release.id}>
          <DataRowHead>{releaseNameText(release, project)}</DataRowHead>
          <DataCell label={t("releases.release-date")}>{releaseDateText(release)}</DataCell>
          <DataCell {...(release.releasedOn === null ? {} : { label: t("releases.released-on") })}>
            {releasedOnText(release)}
          </DataCell>
          <DataCell label={t("releases.issues")}>
            <Issues count={plannedFor(release.id)} project={project} />
          </DataCell>
          <DataCell variant={dataCellVariant.action}>
            {release.mayManage ? (
              <Button
                look={buttonLook.quiet}
                aria-label={t("releases.change-name", { name: release.name })}
                onPress={() => {
                  onChange(release);
                }}
              >
                {t("releases.change")}
              </Button>
            ) : null}
          </DataCell>
        </DataRow>
      ))}
    </DataTable>
  );
}
