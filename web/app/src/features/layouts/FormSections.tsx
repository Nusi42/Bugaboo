import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";
import { Tab, TabList, TabPanel, Tabs } from "react-aria-components";

import type { FormSection } from "./layouts.ts";
import { tabsOf, type TabGroup } from "./form-tabs.ts";

export interface FormSectionsProps {
  /** The sections to draw: the ones of the layout, or the part of them a place shows. */
  readonly sections: readonly FormSection[];
  /** Draws one field by its key. */
  readonly render: (field: string) => ReactNode;
  /** Whether the titles of the sections are shown. */
  readonly titles: boolean;
}

/** One section: its title, when it has one and titles are shown, and its fields under each other. */
function SectionView({
  section,
  render,
  titles,
}: {
  readonly section: FormSection;
  readonly render: FormSectionsProps["render"];
  readonly titles: boolean;
}) {
  return (
    <section
      {...partOf(part.formSection)}
      aria-label={section.title === "" ? undefined : section.title}
    >
      {titles && section.title !== "" ? (
        <h3 {...partOf(part.formSectionTitle)}>{section.title}</h3>
      ) : null}
      <div {...partOf(part.stack)}>
        {section.fields.map((field) => (
          <div key={field}>{render(field)}</div>
        ))}
      </div>
    </section>
  );
}

/** The sections of one tab, under each other. */
function Sections({ group, render, titles }: FormSectionsProps & { readonly group: TabGroup }) {
  return (
    <div {...partOf(part.formPanel)}>
      {group.sections.map((section, at) => (
        <SectionView
          key={[section.tab, section.title, at].join("|")}
          section={section}
          render={render}
          titles={titles}
        />
      ))}
    </div>
  );
}

/**
 * The sections of a form as a person sees them (CFG-01): under each other, or in tabs when the
 * layout puts sections under tabs. Which fields a person gets was decided on the server, from
 * their roles and the levels of the fields; this only draws them.
 */
export function FormSections(props: FormSectionsProps) {
  const groups = tabsOf(props.sections);
  const [chosen, setChosen] = useState<string | undefined>();
  const [first] = groups;
  if (first === undefined) {
    return null;
  }
  if (groups.length === 1) {
    return <Sections {...props} group={first} />;
  }
  const open = groups.find((group) => group.tab === chosen) ?? first;
  return (
    <Tabs
      selectedKey={open.tab}
      onSelectionChange={(key) => {
        setChosen(String(key));
      }}
    >
      <TabList aria-label={t("layouts.tabs")} {...partOf(part.formTabs)}>
        {groups.map((group) => (
          <Tab key={group.tab} id={group.tab} {...partOf(part.formTab)}>
            {group.tab === "" ? t("layouts.tab-other") : group.tab}
          </Tab>
        ))}
      </TabList>
      {groups.map((group) => (
        <TabPanel key={group.tab} id={group.tab}>
          <Sections {...props} group={group} />
        </TabPanel>
      ))}
    </Tabs>
  );
}
