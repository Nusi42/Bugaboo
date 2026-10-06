import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Checkbox } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import styles from "./AccessKeys.module.css";
import { projectsOptions } from "./access-keys.ts";
import { permissionSentence, topicGroups, topicHeading } from "./choice.ts";
import type { Draft } from "./draft.ts";

export interface PermissionPickerProps {
  /** The permissions to choose from, in the order of the catalogue, each with its topic. */
  readonly choices: readonly Schema<"PermissionChoiceBody">[];
  readonly draft: Draft;
}

/** The single permissions a key can be limited to, a group for each topic. */
export function PermissionPicker({ choices, draft }: PermissionPickerProps) {
  return (
    <div className={styles.stack}>
      {topicGroups(choices).map((group) => (
        <fieldset key={group.topic} className={styles.group}>
          <legend className={styles.legend}>{t(topicHeading(group.topic))}</legend>
          {group.permissions.map((key) => (
            <Checkbox
              key={key}
              label={t(permissionSentence(key))}
              isSelected={draft.picked.includes(key)}
              onChange={(on) => {
                draft.pick(key, on);
              }}
            />
          ))}
        </fieldset>
      ))}
    </div>
  );
}

export interface ProjectPickerProps {
  readonly draft: Draft;
}

/** The projects a key can be limited to, one box for each. */
export function ProjectPicker({ draft }: ProjectPickerProps) {
  const projects: readonly Schema<"ProjectChoiceItem">[] =
    useQuery(projectsOptions()).data?.items ?? [];
  if (projects.length === 0) {
    return <p className={styles.muted}>{t("access-keys.projects-none")}</p>;
  }
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{t("access-keys.projects")}</legend>
      <p className={styles.muted}>{t("access-keys.projects-help")}</p>
      {projects.map((project) => (
        <Checkbox
          key={project.id}
          label={t("access-keys.project-label", {
            short_name: project.short_name ?? "",
            name: project.name ?? "",
          })}
          isSelected={draft.projects.includes(project.id ?? "")}
          onChange={(on) => {
            draft.limitTo(project.id ?? "", on);
          }}
        />
      ))}
    </fieldset>
  );
}
