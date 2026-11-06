import { useNavigate, useSearch } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { OnlyWith } from "./OnlyWith.tsx";
import { OthersScreen } from "./others-screen.ts";

/** What the two pages of *Working hours of others* do with the address: choose a person, or a week. */
export function TimeOthers({
  path,
}: {
  readonly path: typeof paths.timeOthers | typeof paths.timeOfPerson;
}) {
  const { week } = useSearch({ strict: false });
  const person = useAddressPart(routeParam.person);
  const navigate = useNavigate();
  return (
    <OnlyWith path={path}>
      <OthersScreen
        person={person}
        week={week}
        onWeek={(first: string) => {
          void navigate({
            to: paths.timeOfPerson,
            params: { person: person ?? "" },
            search: { week: first },
          });
        }}
        onPerson={(chosen: string) => {
          void navigate({ to: paths.timeOfPerson, params: { person: chosen }, search: { week } });
        }}
      />
    </OnlyWith>
  );
}
