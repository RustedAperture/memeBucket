import { LegalPage, LegalSection } from "@/components/legal-page";

export default function LicensePage() {
  return (
    <LegalPage
      title="License"
      description="ezGif is licensed under the GNU General Public License version 3."
      updated="Jun 10, 2026"
    >
      <LegalSection title="GNU General Public License v3.0">
        <p>
          The full GPLv3 license text is included in the repository&apos;s{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 text-foreground">LICENSE</code>{" "}
          file.
        </p>
        <p>
          You can also read the canonical license text from the Free Software
          Foundation at{" "}
          <a
            href="https://www.gnu.org/licenses/gpl-3.0.en.html"
            className="text-foreground underline underline-offset-4"
          >
            gnu.org/licenses/gpl-3.0.en.html
          </a>
          .
        </p>
      </LegalSection>
    </LegalPage>
  );
}
