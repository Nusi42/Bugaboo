import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { CornersAndFaces } from "./CornersAndFaces.tsx";
import { GroupSummary } from "./GroupSummary.tsx";
import { ShadowLayers, type ShadowLayersProps } from "./ShadowLayers.tsx";

/**
 * *Shadow, corners and fonts* (UX §5.6, M8-Q10): the corners of controls and of cards, the faces of
 * text, headings and long text among the built-in ones, and the shadow as layers. Nothing else: no
 * spacing and no font files. It starts closed.
 */
export function ShapeGroup(props: ShadowLayersProps) {
  return (
    <details {...partOf(part.themeGroup)}>
      <GroupSummary
        title={t("theme-editor.group.shape")}
        note={t("theme-editor.shape-changed", {
          count:
            Object.keys(props.values.radii ?? {}).length +
            Object.keys(props.values.fonts ?? {}).length +
            (props.values.shadow === undefined || props.values.shadow === null ? 0 : 1),
          theme: props.startsFrom,
        })}
      />
      <div {...partOf(part.themeGroupBody)}>
        <CornersAndFaces values={props.values} accepts={props.accepts} onChange={props.onChange} />
        <ShadowLayers {...props} />
      </div>
    </details>
  );
}
