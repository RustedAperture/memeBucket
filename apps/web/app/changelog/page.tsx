import { Fragment } from "react";
import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";
import changelogData from "@/public/changelog.json";

type ChangelogSection = {
  heading: string;
  items: string[];
};

type ChangelogEntry = {
  version: string;
  date: string;
  sections: ChangelogSection[];
};

const entries = changelogData as ChangelogEntry[];

export default function ChangelogPage() {
  return (
    <LegalPage
      title="Changelog"
      description="Notable changes to memeBucket."
      updated={entries[0]?.date}
    >
      {entries.map((entry) => (
        <LegalSection key={entry.version} title={`v${entry.version} - ${entry.date}`}>
          {entry.sections.map((section, index) => (
            <Fragment key={index}>
              <h3 className={index > 0 ? "font-medium text-foreground mt-4" : "font-medium text-foreground"}>
                {section.heading}
              </h3>
              <LegalList>
                {section.items.map((item, itemIndex) => (
                  <li key={itemIndex} dangerouslySetInnerHTML={{ __html: item }} />
                ))}
              </LegalList>
            </Fragment>
          ))}
        </LegalSection>
      ))}
    </LegalPage>
  );
}
