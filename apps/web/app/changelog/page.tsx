import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";

export default function ChangelogPage() {
  return (
    <LegalPage
      title="Changelog"
      description="Notable changes to ezGif."
      updated="Jun 10, 2026"
    >
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
