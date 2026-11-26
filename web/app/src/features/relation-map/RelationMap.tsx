import { part, partOf, useLayout } from "@bugaboo/ui";

import { pictureOf, type MapAnswer, type MapPicture } from "./layout.ts";
import { useLooks, type KindPages, type Looks } from "./line-look.ts";
import type { KindsShown } from "./map-data.ts";
import { mapSizes, useMapView, type MapView } from "./map-view.ts";
import { MapList } from "./MapList.tsx";
import { MapNotes, PhoneChain } from "./MapNotes.tsx";
import { MapPicture as Picture } from "./MapPicture.tsx";
import { LineLegend, MapSide } from "./MapSide.tsx";
import { MapTools, type MapToolsProps, type StepsTool } from "./MapTools.tsx";

export interface RelationMapProps {
  readonly answer: MapAnswer;
  /** The pages of the kinds of relation read so far. */
  readonly kindPages: KindPages;
  /** What the picture is called for screen readers, with how many issues it holds. */
  readonly label: (count: number) => string;
  /** What the map says when it was cut at the most issues it holds. */
  readonly cut: string;
  /** What the map says when it holds no relation of the kinds shown. */
  readonly nothing: string;
  /** *Steps away*, on the map of an issue; a search draws its results and walks no steps. */
  readonly steps?: StepsTool;
  /** What stands first in the tools: *Show as* of a search. */
  readonly start?: MapToolsProps["start"];
  readonly shown: KindsShown;
  readonly onShown: (shown: KindsShown) => void;
  /** Makes an issue the start of a new map, by its readable ID (*Start from here*). */
  readonly onStartFrom: (key: string) => void;
}

/** The picture or the list, as the person chose. */
function ShownForm({
  view,
  picture,
  looks,
  label,
  answer,
}: Pick<RelationMapProps, "label" | "answer"> & {
  readonly view: MapView;
  readonly picture: MapPicture;
  readonly looks: Looks;
}) {
  return view.form === "picture" ? (
    <Picture
      {...view.pointing}
      picture={picture}
      sizes={mapSizes()}
      looks={looks}
      zoom={view.zoom}
      pointed={view.pointed}
      chosen={view.chosen}
      label={label(answer.issues.length)}
      fits={view.fits}
      onFit={view.onFit}
    />
  ) : (
    <MapList picture={picture} />
  );
}

/**
 * The relation map of an answer (ISS-16, UX §4.15), of an issue or of a search: the tools, the
 * notes, and the picture with its side, or the list form. The picture is laid out once for the
 * answer (ADR-0009, amendment 2026-11-24) and changes only with it. A phone starts with the list.
 */
export function RelationMap(props: RelationMapProps) {
  const { answer, onStartFrom } = props;
  const phone = useLayout() === "phone";
  const view = useMapView(phone);
  const { kinds, looks } = useLooks(props.kindPages);
  const picture = pictureOf(answer);
  const shown = <ShownForm {...props} view={view} picture={picture} looks={looks} />;
  const chain = { picture, chosen: view.chosen, onStartFrom };
  return (
    <div {...partOf(part.map)}>
      <MapTools
        {...props}
        kinds={kinds}
        looks={looks}
        form={view.form}
        onForm={view.setForm}
        zoom={view.zoom}
        onZoom={view.setZoom}
        onFit={view.onFitPress}
        phone={phone}
      />
      <MapNotes {...props} picture={picture} />
      {phone ? (
        <>
          {shown}
          {view.form === "picture" ? <LineLegend kinds={kinds} looks={looks} /> : null}
          <PhoneChain {...chain} onClose={view.unchoose} />
        </>
      ) : (
        <div {...partOf(part.mapBody)}>
          {shown}
          <MapSide {...chain} kinds={kinds} looks={looks} />
        </div>
      )}
    </div>
  );
}
