import { dragKinds } from "../../shared/drag-kinds.ts";
import { BoxContent } from "./BoxContent.tsx";
import { BoxFrame } from "./BoxFrame.tsx";
import { BoxMenu } from "./BoxMenu.tsx";
import { useBoxAnswer, type Box, type Dashboard } from "./dashboards.ts";
import type { GridControls } from "./use-grid-placing.ts";

export interface GridBoxProps {
  readonly dashboard: Dashboard;
  readonly box: Box;
  readonly at: number;
  readonly changing: boolean;
  readonly grid: GridControls;
  readonly onChangeBox: (box: Box) => void;
}

/** One box of the grid with what its read answers for the person who looks at it, and in *Change* its menu. */
export function GridBox({ dashboard, box, at, changing, grid, onChangeBox }: GridBoxProps) {
  const read = useBoxAnswer(dashboard.id, box.id);
  return (
    <BoxFrame
      box={box}
      placing={{
        drag: grid.draggable
          ? {
              move: grid.handlers(at, dragKinds.dashboardBox.kind),
              resize: grid.handlers(at, dragKinds.dashboardCorner.kind),
            }
          : undefined,
        dragging: grid.dragged === at,
        keys: grid.keyed === at,
      }}
      menu={
        changing ? (
          <BoxMenu title={box.title} actions={grid.actionsOf(box, at, onChangeBox)} />
        ) : null
      }
    >
      <BoxContent box={box} read={read} />
    </BoxFrame>
  );
}
