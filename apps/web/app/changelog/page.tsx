import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";

export default function ChangelogPage() {
  return (
    <LegalPage
      title="Changelog"
      description="Notable changes to ezGif."
      updated="Jun 11, 2026"
    >
      <LegalSection title="v0.1.2 - Jun 11, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added an "Add to Pool" Discord message context menu command to save images directly from messages into an "Added from Discord" pool.</li>
          <li>Added the ability to rename image pools.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Migrated the web dashboard's sidebar layout to use standard Shadcn UI components.</li>
          <li>Consolidated pool settings (rename, delete, unsubscribe) into a clean Settings modal.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.1 - Jun 11, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added homepage buttons for Ko-fi support and the GitHub repository.</li>
          <li>Added a footer theme selector with Light, Dark, and Auto modes.</li>
          <li>Added drag-and-drop support and a modal dropdown for moving images between pools.</li>
          <li>Added a GIF search feature powered by the Klipy API, accessible directly from the pool image form.</li>
        </LegalList>

        <h3 className="font-medium text-foreground">Fixed</h3>
        <LegalList>
          <li>Fixed theme selector styling so only one mode appears selected at a time.</li>
          <li>Fixed an issue in GIF search where "Load more" would append duplicate results.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Improved the GIF search layout by using a masonry-style columns layout to better preserve image aspect ratios.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.0 - Jun 10, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Initial ezGif Discord user app and web dashboard.</li>
          <li>Discord OAuth sign-in and session-backed account management.</li>
          <li>Personal media pools for organizing image and GIF URLs.</li>
          <li>Discord commands for creating pools, adding images, listing pools, opening the dashboard, and sending random images.</li>
          <li>Web dashboard for managing pools, images, notes, and account username.</li>
          <li>Pool sharing with share links, subscriptions, subscriber counts, and optional whitelist access.</li>
          <li>Account export endpoint for owned pools and image URLs.</li>
          <li>Account deletion endpoint for account-linked data.</li>
          <li>Terms of Service, Privacy Policy, Changelog, and GPLv3 License pages.</li>
        </LegalList>

        <h3 className="font-medium text-foreground">Privacy and Security</h3>
        <LegalList>
          <li>Discord user identity is stored as an HMAC-SHA256 user key rather than a raw Discord user ID.</li>
          <li>State-changing dashboard requests use CSRF protection.</li>
          <li>Selected routes use rate limiting.</li>
        </LegalList>
      </LegalSection>
    </LegalPage>
  );
}
