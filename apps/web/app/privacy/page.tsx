import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";

export default function PrivacyPage() {
  return (
    <LegalPage
      title="Privacy Policy"
      description="What ezGif collects, stores, and uses to provide Discord media pools."
      updated="Jun 10, 2026"
    >
      <LegalSection title="Data We Collect">
        <LegalList>
          <li>Account identity, including an internal user ID and HMAC-SHA256 key derived from your Discord user ID.</li>
          <li>Discord profile display data returned by OAuth, such as display name and avatar URL.</li>
          <li>Your ezGif username, used for sharing and whitelist features.</li>
          <li>Session records, CSRF token hashes, OAuth state cookies, and expiration timestamps.</li>
          <li>Pool names, image or GIF URLs, optional notes, creation timestamps, and related internal IDs.</li>
          <li>Share tokens, subscriptions, subscriber counts, whitelist settings, and whitelist membership.</li>
          <li>Random-send history, including pool name, selected URL, visibility setting, and timestamp.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="How We Use Data">
        <p>
          ezGif uses this data to authenticate you with Discord, show and manage your
          pools, send random media through Discord commands, support sharing and
          whitelist features, maintain sessions and CSRF protection, apply rate
          limiting, and process export or deletion requests.
        </p>
      </LegalSection>

      <LegalSection title="Sharing and Visibility">
        <p>
          Private pools are intended to be visible only to you. If you create a share
          link or allow subscriptions, users with access may see pool names, image
          URLs, previews, notes, your ezGif username, and subscriber-related
          information. Discord message recipients can see media you send according
          to the message context and visibility option you choose.
        </p>
      </LegalSection>

      <LegalSection title="Third Parties">
        <p>
          ezGif relies on Discord for OAuth, application commands, profile data, and
          message delivery. Image and GIF URLs may point to third-party hosts; loading
          or viewing them may contact those hosts. Those services have their own
          terms and privacy policies.
        </p>
      </LegalSection>

      <LegalSection title="Retention, Export, and Deletion">
        <p>
          ezGif keeps account, pool, image, sharing, subscription, whitelist, and
          command history data until it is deleted, the account is deleted, or the
          maintainers remove it for operational reasons. When signed in, you can
          export owned pools and image URLs from the dashboard. Account deletion
          removes the user record and account-linked data removed through database
          cascade rules.
        </p>
      </LegalSection>

      <LegalSection title="Security">
        <p>
          ezGif uses a keyed hash for Discord identity instead of intentionally
          storing raw Discord user IDs. The dashboard uses secure session cookies,
          CSRF protection for state-changing requests, and rate limiting on selected
          routes. No system can guarantee perfect security.
        </p>
      </LegalSection>

      <LegalSection title="Changes and Contact">
        <p>
          This policy may be updated as ezGif changes. Questions or deletion/export
          concerns can be sent through the project repository or maintainer contact
          listed in the README.
        </p>
      </LegalSection>
    </LegalPage>
  );
}
