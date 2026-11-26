import { useRef, useState, type ReactNode } from "react";

import { DataCell, DataRow, DataRowHead, DataTable } from "./DataTable.tsx";
import { partOf } from "./parts.ts";
import { usePlace } from "./place.ts";
import { Switch } from "./Switch.tsx";
import { part } from "./tokens.ts";

/** One value of a chart: a bar, or the one number. */
export interface ChartRow {
  /** Tells the rows apart; unique in the chart. */
  readonly key: string;
  /** What the value is of: the name of a group, *Done*, *Time left*. */
  readonly name: string;
  /** The value as a number, which sets the length of its bar. */
  readonly value: number;
  /** The value as people read it, written at its bar and in the table: *12*, *3h 30m*. */
  readonly text: string;
  /** What the value is over, small under a large number and after the value in the table: *over 9 of 14 issues*. */
  readonly detail?: string | undefined;
}

/** The words of a chart: its name, the names of the two columns of its table, and the switch to the table. */
export interface ChartLabels {
  /** The name of the chart and of its table, for screen readers. */
  readonly name: string;
  /** The heading of the column of names: the field the values are grouped by. */
  readonly names: string;
  /** The heading of the column of values: *Issues*. */
  readonly values: string;
  /** The switch that shows the table instead of the chart: *Show as table*. */
  readonly asTable: string;
}

/** The share of the largest value a bar takes, in percent; a bar of a value above none is never shorter than a sliver. */
function shareOf(value: number, largest: number): number {
  return largest <= 0 ? 0 : Math.round((Math.max(0, value) / largest) * 100);
}

/** One bar: its name before it, its value written at its end; the length is a share of the largest value. */
function Bar({ row, largest }: { readonly row: ChartRow; readonly largest: number }) {
  const bar = useRef<HTMLSpanElement>(null);
  usePlace(bar, { "chart-share": shareOf(row.value, largest) });
  return (
    <li {...partOf(part.chartBar)}>
      <span {...partOf(part.chartBarName)}>{row.name}</span>
      <span {...partOf(part.chartBarLane)}>
        <span ref={bar} {...partOf(part.chartBarFill)} />
        <span {...partOf(part.chartBarValue)}>{row.text}</span>
      </span>
    </li>
  );
}

/** The table of a chart: a row for each value, the name as the header of its row. */
function ChartTable({
  rows,
  labels,
}: {
  readonly rows: readonly ChartRow[];
  readonly labels: ChartLabels;
}) {
  return (
    <DataTable label={labels.name} columns={[labels.names, labels.values]}>
      {rows.map((row) => (
        <DataRow key={row.key}>
          <DataRowHead>{row.name}</DataRowHead>
          <DataCell label={labels.values}>
            {row.detail === undefined ? row.text : `${row.text} ${row.detail}`}
          </DataCell>
        </DataRow>
      ))}
    </DataTable>
  );
}

/**
 * A chart and its table (CFG-03, M10-Q13): the picture is for the eye only; the table is what screen readers read, and
 * *Show as table* shows it instead of the picture. The switch stands first, before the picture or the table, so it
 * keeps its place when it turns and nothing moves under the pointer.
 */
function ChartFrame({
  rows,
  labels,
  picture,
}: {
  readonly rows: readonly ChartRow[];
  readonly labels: ChartLabels;
  readonly picture: ReactNode;
}) {
  const [table, setTable] = useState(false);
  return (
    <figure {...partOf(part.chart)} aria-label={labels.name}>
      <div {...partOf(part.chartSwitch)}>
        <Switch label={labels.asTable} isSelected={table} onChange={setTable} />
      </div>
      {table ? null : (
        <div {...partOf(part.chartPicture)} aria-hidden>
          {picture}
        </div>
      )}
      <div {...partOf(table ? part.chartTable : part.visuallyHidden)}>
        <ChartTable rows={rows} labels={labels} />
      </div>
    </figure>
  );
}

export interface BarChartProps {
  /** The values, in the order they stand: the order of the groups, never of their size (ADR-0050, decision 5). */
  readonly rows: readonly ChartRow[];
  readonly labels: ChartLabels;
  /** What stands instead of the bars when there are none: *No issues.* */
  readonly none: string;
}

/**
 * Horizontal bars with the value written at each bar (M10-Q13), in the order given. A bar is told apart by its name
 * and its value, never by its color alone; every bar has the one color of the chart.
 */
export function BarChart({ rows, labels, none }: BarChartProps) {
  const largest = Math.max(0, ...rows.map((row) => row.value));
  if (rows.length === 0) {
    return <p {...partOf(part.muted)}>{none}</p>;
  }
  return (
    <ChartFrame
      rows={rows}
      labels={labels}
      picture={
        <ul {...partOf(part.chartBars)}>
          {rows.map((row) => (
            <Bar key={row.key} row={row} largest={largest} />
          ))}
        </ul>
      }
    />
  );
}

export interface NumberChartProps {
  /** The numbers, each large with its name under it: one, or a few side by side. */
  readonly rows: readonly ChartRow[];
  readonly labels: ChartLabels;
}

/** One large number with its name (M10-Q13), or a few side by side, with the table of them. */
export function NumberChart({ rows, labels }: NumberChartProps) {
  return (
    <ChartFrame
      rows={rows}
      labels={labels}
      picture={
        <ul {...partOf(part.chartNumbers)}>
          {rows.map((row) => (
            <li key={row.key} {...partOf(part.chartNumber)}>
              <span {...partOf(part.chartNumberValue)}>{row.text}</span>
              {row.detail === undefined ? null : (
                <span {...partOf(part.chartNumberName)}>{row.detail}</span>
              )}
              <span {...partOf(part.chartNumberName)}>{row.name}</span>
            </li>
          ))}
        </ul>
      }
    />
  );
}
