import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, Skeleton, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { ProjectGate, type OneProject } from "../projects/index.ts";
import { ReleaseTable, type ReleaseTableProps } from "./ReleaseTable.tsx";
import { ReleaseWindow } from "./ReleaseWindow.tsx";
import { stateText } from "./release-words.ts";
import { useReleaseCounts, useReleases, type Release, type StateWord } from "./releases.ts";

/** What is open over the page: the window of a release, or of a new one. */
type Opened = { readonly release: Release | undefined } | null;

/** The states in the order they are drawn: *Retired* comes last and is folded. */
const states: readonly StateWord[] = ["planned", "released", "retired"];

interface StateProps extends Omit<ReleaseTableProps, "label"> {
  readonly state: StateWord;
}

/** The releases of one state with their heading; the retired ones are folded until they are asked for. */
function OneState({ state, releases, ...table }: StateProps) {
  const [shown, setShown] = useState(state !== "retired");
  if (releases.length === 0) {
    return null;
  }
  const heading = t("releases.state-heading", { state: stateText(state), count: releases.length });
  return (
    <section {...partOf(part.stack)} aria-label={stateText(state)}>
      <h2 {...partOf(part.releaseHeading)}>
        {state === "retired" ? (
          <Button
            look={buttonLook.quiet}
            aria-expanded={shown}
            onPress={() => {
              setShown(!shown);
            }}
          >
            {heading}
          </Button>
        ) : (
          heading
        )}
      </h2>
      {shown ? <ReleaseTable label={stateText(state)} releases={releases} {...table} /> : null}
    </section>
  );
}

interface ListsProps {
  readonly project: string;
  readonly id: string;
  readonly onChange: (release: Release) => void;
}

/** What the page lists: the releases by state, or why there are none yet. */
function Lists({ project, id, onChange }: ListsProps) {
  const list = useReleases(id);
  const planned = useReleaseCounts(id, "planned for release");
  return (
    <>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {list.isPending || planned.isPending ? <Skeleton lines={3} /> : null}
      {!list.isPending && list.releases.length === 0 ? (
        <p {...partOf(part.muted)}>{t("releases.none")}</p>
      ) : null}
      {list.isPending || planned.isPending
        ? null
        : states.map((state) => (
            <OneState
              key={state}
              state={state}
              releases={list.releases.filter((release) => release.state === state)}
              project={project}
              plannedFor={(release) => planned.data?.get(release)}
              onChange={onChange}
            />
          ))}
    </>
  );
}

/** The window of a release over the page, with the issues that name it, as the reader sees them. */
function Opening({
  opened,
  project,
  onClose,
}: {
  readonly opened: NonNullable<Opened>;
  readonly project: { readonly id: string; readonly name: string };
  readonly onClose: () => void;
}) {
  const planned = useReleaseCounts(project.id, "planned for release");
  const found = useReleaseCounts(project.id, "found in release");
  const { release } = opened;
  return (
    <ReleaseWindow
      project={project}
      release={release}
      counts={
        release === undefined
          ? undefined
          : {
              planned: planned.data?.get(release.id)?.issues ?? 0,
              found: found.data?.get(release.id)?.issues ?? 0,
            }
      }
      onClose={onClose}
    />
  );
}

/** The page of the releases once the project is known. */
function Releases({ project }: { readonly project: OneProject }) {
  const [opened, setOpened] = useState<Opened>(null);
  const { id, short_name: shortName, name } = project.project;
  const mayManage = project.permissions.includes("manage-releases");
  return (
    <>
      <Card
        heading={t("releases.title", { project: name })}
        level={1}
        intro={t("releases.intro")}
        actions={
          mayManage ? (
            <Button
              look={buttonLook.primary}
              onPress={() => {
                setOpened({ release: undefined });
              }}
            >
              {t("releases.create")}
            </Button>
          ) : undefined
        }
      >
        <Lists
          project={shortName}
          id={id}
          onChange={(release) => {
            setOpened({ release });
          }}
        />
      </Card>
      {opened === null ? null : (
        <Opening
          opened={opened}
          project={{ id, name }}
          onClose={() => {
            setOpened(null);
          }}
        />
      )}
    </>
  );
}

/**
 * `/p/WEB/releases` (UX §4.16, ISS-08, ADR-0049): the releases offered in a project, those made in it
 * and those that reach it from above, by state (*Retired* folded), each with its *Release date* and
 * the number of issues planned for it that the reader sees. Those who hold *Manage releases* make and
 * change them; for everybody else the page is read only.
 */
export function ReleasesPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} page={projectPages.releases}>
      {(project) => <Releases project={project} />}
    </ProjectGate>
  );
}
