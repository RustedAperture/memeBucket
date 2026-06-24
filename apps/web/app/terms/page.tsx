import Link from "next/link";
import { LegalList, LegalPage, LegalSection } from "@/components/legal-page";

export default function TermsPage() {
  return (
    <LegalPage
      title="Terms of Service"
      description="Rules for using memeBucket, the Discord app and dashboard for personal image and GIF buckets."
      updated="Jun 10, 2026"
    >
      <LegalSection title="Acceptance">
        <p>
          These Terms govern your use of memeBucket. By installing, authorizing, accessing,
          or using memeBucket, you agree to these Terms. If you do not agree, do not use
          the service.
        </p>
      </LegalSection>

      <LegalSection title="Acceptable Use">
        <LegalList>
          <li>Do not use memeBucket for unlawful, abusive, harassing, deceptive, or harmful activity.</li>
          <li>Do not store, share, or send content that violates Discord rules or applicable law.</li>
          <li>Do not disrupt, scrape, overload, reverse engineer, or bypass service access controls.</li>
          <li>You are responsible for the URLs, notes, bucket names, and content you add.</li>
        </LegalList>
      </LegalSection>

      <LegalSection title="Discord Integration">
        <p>
          memeBucket uses Discord OAuth and Discord application commands. Discord is a
          separate service with its own terms and policies, and you are responsible
          for following them when using memeBucket.
        </p>
      </LegalSection>

      <LegalSection title="User Content">
        <p>
          You retain any rights you have in content you add. By adding content, you
          grant memeBucket permission to store, display, process, and send it as needed
          to provide the service. Shared buckets may expose bucket names, image URLs,
          previews, notes, owner usernames, and subscriber information to users who
          can access the share.
        </p>
      </LegalSection>

      <LegalSection title="Data and Privacy">
        <p>
          memeBucket stores account, session, bucket, image URL, notes, sharing,
          subscription, whitelist, and command usage data as described in the{" "}
          <Link href="/privacy" className="text-foreground underline underline-offset-4">
            Privacy Policy
          </Link>
          .
        </p>
      </LegalSection>

      <LegalSection title="Availability and Termination">
        <p>
          memeBucket may change, break, or become unavailable at any time. The maintainers
          may suspend or terminate access for abuse, security risk, policy violations,
          or operational reasons. Export and deletion tools are available from the
          account area of the dashboard when signed in.
        </p>
      </LegalSection>

      <LegalSection title="Disclaimers">
        <p>
          memeBucket is provided &quot;as is&quot; and &quot;as available&quot; without warranties of any
          kind. The maintainers are not responsible for user content, third-party
          media URLs, Discord changes or outages, data loss, interruptions, or damage
          resulting from use of the service.
        </p>
      </LegalSection>

      <LegalSection title="Changes and Contact">
        <p>
          These Terms may be updated over time. Continued use after changes are
          posted means you accept the updated Terms. Questions can be sent through
          the project repository or maintainer contact listed in the README.
        </p>
      </LegalSection>
    </LegalPage>
  );
}
