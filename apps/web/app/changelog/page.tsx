import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";

export default function ChangelogPage() {
  return (
    <LegalPage
      title="Changelog"
      description="Notable changes to memeBucket."
      updated="Jun 25, 2026"
    >
      <LegalSection title="v0.1.6 - Jun 25, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added bulk editing features to the bucket view, including <strong>Bulk Delete</strong> and <strong>Bulk Move</strong> with efficient transactional backend endpoints.</li>
          <li>Added <strong>Bulk Add Links</strong> allowing users to paste multiple image/video URLs at once with a step-by-step progress UI and error recovery (pre-filling with only failed URLs for easy retries).</li>
          <li>Added batch actions to <strong>Select All</strong> and <strong>Copy Links</strong> for all selected items.</li>
          <li>Added <strong>Import Data</strong> functionality in the Account Settings menu, allowing users to upload their exported backup JSON files to easily restore or duplicate their buckets and media with all metadata and tags fully preserved.</li>
          <li>Added client-side format validation and server-side duplicate prevention to ensure seamless imports.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Rebranded the Discord system bucket from <strong>Added from Discord</strong> to <strong>Inbox</strong>, including a database migration to automatically migrate existing records.</li>
          <li>Removed redundant drag count badges from selected cards for a cleaner visual aesthetic.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Fixed</h3>
        <LegalList>
          <li>Fixed the <strong>Export Data</strong> functionality which was failing due to outdated database table references (<code>pools</code>) from the rebranding.</li>
          <li>Updated the export data payload to include all newly added image metadata fields (title, favorite, random weight, tags, notes, and created_at).</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.5 - Jun 23, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added mobile-specific touch gestures: single tap to copy media URLs to the clipboard, and long press (500ms) to open the detailed image properties modal in both the Bucket and Search views.</li>
          <li>Enabled clicking images on desktop viewports in the Search page to open the properties dialog, matching the Bucket view behavior.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Rebranded the entire application from <strong>ezGif</strong> to <strong>memeBucket</strong>, renaming all references to <strong>pools</strong> to <strong>buckets</strong> throughout the database, backend services, Discord commands, and frontend UI.</li>
          <li>Refactored the Bucket control toolbar to use a responsive two-row layout on mobile viewports, allowing the size slider to remain fully functional while the input box stretches to fill the screen width.</li>
          <li>Configured Sonner toast notifications to dynamically display at the top center of the screen on mobile devices.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.4 - Jun 22, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added an optional <code>target</code> parameter to the <code>/ez</code> slash command to send the GIF directly to a specific user.</li>
          <li>Added a "Reply with GIF" right-click message context menu command to instantly send a GIF directed at the author of the selected message.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Fixed</h3>
        <LegalList>
          <li>Fixed an issue causing right-click "Reply with GIF" modals to time out due to slow database reads by enabling SQLite WAL mode.</li>
          <li>Fixed a Discord API <code>400 Bad Request</code> error preventing the modal from opening by removing unsupported select menus.</li>
          <li>Added a friendly error message listing available buckets if an invalid bucket name is entered in the modal.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Updated Discord integration to embed GIFs so URLs are hidden instead of using zero-width spaces.</li>
          <li>Restored the user's specific Discord profile accent color to embeds sent from the bot.</li>
          <li>Updated dependencies to patch security vulnerabilities.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.3 - Jun 14, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added Library search for saved GIFs and images across accessible buckets, with filters for tags, bucket, favorites, and random-enabled state.</li>
          <li>Added image metadata fields for title, tags, favorite status, random weight, and notes.</li>
          <li>Added metadata editing from image details and bulk editing for selected images.</li>
          <li>Added Klipy metadata suggestions so saved GIFs can start with a title and suggested tags.</li>
          <li>Added a Library card to the dashboard.</li>
          <li>Added a "Disable usage" toggle to image buckets.</li>
          <li>Auto-injected a read-only "Favorites" bucket containing all starred media.</li>
          <li>Expanded the "Paste URL" field to natively double as a Klipy GIF search query.</li>
          <li>Added a star when hovering over an image to easily toggle favorite status.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Improved random image selection with per-image weights and stronger recent-repeat avoidance.</li>
          <li>Renamed the global saved-media search surface to Library to distinguish it from searching Klipy for new GIFs.</li>
          <li>Expanded access checks and tests for library search across owned, subscribed, public, private, and whitelisted buckets.</li>
          <li>Refactored bucket view and search pages to share a unified responsive layout.</li>
          <li>System buckets (like Favorites or Inbox) are now automatically hidden when empty.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.2 - Jun 11, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added an "Add to Bucket" Discord message context menu command to save images directly from messages into an "Inbox" bucket.</li>
          <li>Added the ability to rename image buckets.</li>
        </LegalList>

        <h3 className="font-medium text-foreground mt-4">Changed</h3>
        <LegalList>
          <li>Migrated the web dashboard's sidebar layout to use standard Shadcn UI components.</li>
          <li>Consolidated bucket settings (rename, delete, unsubscribe) into a clean Settings modal.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="v0.1.1 - Jun 11, 2026">
        <h3 className="font-medium text-foreground">Added</h3>
        <LegalList>
          <li>Added homepage buttons for Ko-fi support and the GitHub repository.</li>
          <li>Added a footer theme selector with Light, Dark, and Auto modes.</li>
          <li>Added drag-and-drop support and a modal dropdown for moving images between buckets.</li>
          <li>Added a GIF search feature powered by the Klipy API, accessible directly from the bucket image form.</li>
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
          <li>Initial memeBucket Discord user app and web dashboard.</li>
          <li>Discord OAuth sign-in and session-backed account management.</li>
          <li>Personal media buckets for organizing image and GIF URLs.</li>
          <li>Discord commands for creating buckets, adding images, listing buckets, opening the dashboard, and sending random images.</li>
          <li>Web dashboard for managing buckets, images, notes, and account username.</li>
          <li>Bucket sharing with share links, subscriptions, subscriber counts, and optional whitelist access.</li>
          <li>Account export endpoint for owned buckets and image URLs.</li>
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
