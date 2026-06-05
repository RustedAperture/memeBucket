import { AppShell } from "@/components/app-shell";

export default function HomePage() {
  return (
    <AppShell>
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold">Dashboard</h1>
        <p className="text-muted-foreground">Manage personal Discord media categories.</p>
      </div>
    </AppShell>
  );
}
