import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  Checkbox,
  ChoiceGroup,
  Form,
  SubmitButton,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { choiceName, projectChoicesOptions } from "../my-settings/index.ts";
import type { ConnectRequest } from "./request.ts";
import { useDecision } from "./use-decision.ts";

export interface ConnectFormProps {
  readonly request: ConnectRequest;
  /** What the app asked for. The person may allow less, never more. */
  readonly asked: Schema<"KeyChoice">;
  readonly onThisComputer: boolean;
}

/** What an app can be given: the two plain choices of an access key. */
type Scope = "see-only" | "see-and-change";

/** The two ways an app can be limited. */
type Where = "all" | "some";

/**
 * The scopes the form offers, what the app asked first and *See only* below it. An app that asked
 * to see only is not offered more (ADR-0042, the consent step).
 */
function offered(asked: Schema<"KeyChoice">): readonly Scope[] {
  return asked === "see-and-change" ? ["see-and-change", "see-only"] : ["see-only"];
}

interface ProjectsProps {
  readonly picked: readonly string[];
  readonly onPick: (picked: readonly string[]) => void;
  readonly where: Where;
  readonly onWhere: (where: Where) => void;
}

/** *In which projects*: every project the person sees, or only some, each with a box. */
function Projects({ picked, onPick, where, onWhere }: ProjectsProps) {
  const projects: readonly Schema<"ProjectChoiceItem">[] =
    useQuery(projectChoicesOptions()).data?.items ?? [];
  if (projects.length === 0) {
    return null;
  }
  return (
    <>
      <ChoiceGroup
        legend={t("connect.projects")}
        choices={[
          { value: "all", label: t("connect.projects-all") },
          { value: "some", label: t("connect.projects-some") },
        ]}
        value={where}
        onChange={onWhere}
      />
      {where === "some" ? (
        <fieldset {...partOf(part.accessKeysGroup)}>
          <legend {...partOf(part.accessKeysLegend)}>{t("connect.projects-some")}</legend>
          {projects.map((project) => (
            <Checkbox
              key={project.id}
              label={t("access-keys.project-label", {
                short_name: project.short_name ?? "",
                name: project.name ?? "",
              })}
              isSelected={picked.includes(project.id ?? "")}
              onChange={(on) => {
                const id = project.id ?? "";
                onPick(on ? [...picked, id] : picked.filter((each) => each !== id));
              }}
            />
          ))}
        </fieldset>
      ) : null}
    </>
  );
}

/** *Allow* and *Cancel*, with what the app may do and in which projects. */
export function ConnectForm({ request, asked, onThisComputer }: ConnectFormProps) {
  const choices = offered(asked);
  const [scope, setScope] = useState<Scope>(choices[0] ?? "see-only");
  const [where, setWhere] = useState<Where>("all");
  const [picked, setPicked] = useState<readonly string[]>([]);
  const deciding = useDecision(request);
  return (
    <Form
      onSubmit={() => {
        deciding.send({ allow: true, scope, projects: where === "some" ? picked : [] });
      }}
    >
      <p>{t("connect.acts-as")}</p>
      <ChoiceGroup
        legend={t("access-keys.may")}
        choices={choices.map((value) => ({ value, label: t(choiceName(value)) }))}
        value={scope}
        onChange={setScope}
      />
      {asked === "see-only" ? <p {...partOf(part.muted)}>{t("connect.only-see")}</p> : null}
      <Projects picked={picked} onPick={setPicked} where={where} onWhere={setWhere} />
      <p {...partOf(part.muted)}>
        {onThisComputer ? t("connect.where-computer") : t("connect.where-web")}
      </p>
      {deciding.problem === null ? null : <Alert tone={alertTone.danger}>{deciding.problem}</Alert>}
      {deciding.leaving ? (
        <Alert tone={alertTone.success}>
          {t("connect.going-back")} {t("connect.going-back-help")}
        </Alert>
      ) : null}
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={deciding.isPending}>{t("connect.allow")}</SubmitButton>
        <Button
          onPress={() => {
            deciding.send({ allow: false, scope, projects: [] });
          }}
          isDisabled={deciding.isPending || deciding.leaving}
        >
          {t("connect.cancel")}
        </Button>
      </div>
    </Form>
  );
}
