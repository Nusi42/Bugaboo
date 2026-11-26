import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  ChoiceGroup,
  part,
  partOf,
  SelectField,
  selectLook,
} from "@bugaboo/ui";
import type { ReactNode } from "react";
import { ToggleButton } from "react-aria-components";

import type { RelationKindLine } from "../relation-kinds/index.ts";
import type { LineLook, Looks } from "./line-look.ts";
import { LineSwatch } from "./LineSwatch.tsx";
import { stepChoices, type KindsShown, type StepsAway } from "../../shared/relation-map-view.ts";
import { zoomSteps, type MapForm } from "./view-state.ts";

/** The word of a choice of *Steps away*: its number, or *Everything*. */
const everything = "everything";

function stepWord(steps: StepsAway): string {
  return steps === undefined ? everything : String(steps);
}

function stepOf(word: string): StepsAway {
  return stepChoices.find((steps) => stepWord(steps) === word);
}

/** *Steps away*: what is chosen, and how a choice is told. */
export interface StepsTool {
  readonly value: StepsAway;
  readonly onChange: (steps: StepsAway) => void;
}

export interface MapToolsProps {
  /** What stands first: *Show as* and the other choices of the view of a search. */
  readonly start?: ReactNode;
  /** *Steps away*, on the map of an issue; a search draws its results and walks no steps. */
  readonly steps?: StepsTool;
  readonly kinds: readonly RelationKindLine[];
  readonly looks: Looks;
  readonly shown: KindsShown;
  readonly onShown: (shown: KindsShown) => void;
  readonly form: MapForm;
  readonly onForm: (form: MapForm) => void;
  readonly zoom: number;
  readonly onZoom: (zoom: number) => void;
  readonly onFit: () => void;
  /** A phone shows the list first and offers the picture by one button. */
  readonly phone: boolean;
}

/** One chip of *Kinds shown*: the line of the kind and its words; pressing it turns the kind on or off. */
function KindChip({
  word,
  look,
  isSelected,
  onChange,
}: {
  readonly word: string;
  readonly look: LineLook;
  readonly isSelected: boolean;
  readonly onChange: (on: boolean) => void;
}) {
  return (
    <ToggleButton {...partOf(part.mapKind)} isSelected={isSelected} onChange={onChange}>
      <LineSwatch look={look} />
      {word}
    </ToggleButton>
  );
}

/** *Kinds shown* (UX §4.15): a chip for each kind of relation, in the order they are offered in, and *part of*. */
function KindsChips({
  kinds,
  looks,
  shown,
  onShown,
}: Pick<MapToolsProps, "kinds" | "looks" | "shown" | "onShown">) {
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{t("relation-map.kinds-shown")}</legend>
      <div {...partOf(part.choiceGroupChoices)}>
        {kinds.map((kind) => (
          <KindChip
            key={kind.id}
            word={kind.forward}
            look={looks.ofKind(kind.id)}
            isSelected={!shown.hidden.includes(kind.id)}
            onChange={(on) => {
              onShown({
                ...shown,
                hidden: on
                  ? shown.hidden.filter((id) => id !== kind.id)
                  : [...shown.hidden, kind.id],
              });
            }}
          />
        ))}
        <KindChip
          word={t("relation-map.kind-part-of")}
          look={looks.partOf}
          isSelected={shown.partOf}
          onChange={(on) => {
            onShown({ ...shown, partOf: on });
          }}
        />
      </div>
    </fieldset>
  );
}

/** The zoom with its percentage, and *Fit*. */
function Zoom({ zoom, onZoom, onFit }: Pick<MapToolsProps, "zoom" | "onZoom" | "onFit">) {
  return (
    <div {...partOf(part.mapZoom)}>
      <SelectField
        label={t("relation-map.zoom")}
        look={selectLook.inline}
        choices={zoomSteps.map((step) => ({
          value: String(step),
          label: t("relation-map.zoom-level", { percent: Math.round(step * 100) }),
        }))}
        value={String(zoom)}
        onChange={(value) => {
          onZoom(Number(value));
        }}
      />
      <Button look={buttonLook.quiet} onPress={onFit}>
        {t("relation-map.fit")}
      </Button>
    </div>
  );
}

/**
 * The tools over the relation map (UX §4.15): *Steps away* on the map of an issue, *Kinds shown*,
 * *Picture · List*, the zoom with *Fit*, and the note that only issues the person may see are shown.
 */
export function MapTools(props: MapToolsProps) {
  const { start, steps, form, onForm, phone } = props;
  return (
    <div {...partOf(part.mapTools)}>
      {start}
      {steps === undefined ? null : (
        <ChoiceGroup
          legend={t("relation-map.steps-away")}
          choices={stepChoices.map((each) => ({
            value: stepWord(each),
            label: each === undefined ? t("relation-map.everything") : String(each),
          }))}
          value={stepWord(steps.value)}
          onChange={(word) => {
            steps.onChange(stepOf(word));
          }}
        />
      )}
      <KindsChips {...props} />
      {phone ? (
        <Button
          onPress={() => {
            onForm(form === "list" ? "picture" : "list");
          }}
        >
          {form === "list" ? t("relation-map.show-as-picture") : t("relation-map.show-as-list")}
        </Button>
      ) : (
        <ChoiceGroup
          legend={t("relation-map.drawn-as")}
          choices={[
            { value: "picture", label: t("relation-map.picture") },
            { value: "list", label: t("relation-map.list") },
          ]}
          value={form}
          onChange={onForm}
        />
      )}
      {form === "picture" ? <Zoom {...props} /> : null}
      <p {...partOf(part.mapNote)}>{t("relation-map.only-visible")}</p>
    </div>
  );
}
