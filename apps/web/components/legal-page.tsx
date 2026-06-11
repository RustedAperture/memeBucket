import { AppShell } from "@/components/app-shell";

type LegalPageProps = {
  title: string;
  description: string;
  updated?: string;
  children: React.ReactNode;
};

export function LegalPage({ title, description, updated, children }: LegalPageProps) {
  return (
    <AppShell>
      <article className="mx-auto w-full max-w-3xl space-y-8 pb-12">
        <header className="space-y-3 border-b pb-6">
          <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
          <p className="text-base text-muted-foreground">{description}</p>
          {updated ? (
            <p className="text-sm text-muted-foreground">Last updated: {updated}</p>
          ) : null}
        </header>
        <div className="space-y-8 text-sm leading-7 text-foreground">{children}</div>
      </article>
    </AppShell>
  );
}

export function LegalSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3">
      <h2 className="text-xl font-semibold tracking-tight">{title}</h2>
      <div className="space-y-3 text-muted-foreground">{children}</div>
    </section>
  );
}

export function LegalList({ children }: { children: React.ReactNode }) {
  return <ul className="ml-5 list-disc space-y-2 text-muted-foreground">{children}</ul>;
}
