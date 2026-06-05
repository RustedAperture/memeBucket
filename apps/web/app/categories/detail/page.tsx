import { AppShell } from "@/components/app-shell";

export default function CategoryDetailPage() {
  return (
    <AppShell>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold">Category</h1>
        <div id="media-link-list-root" />
      </div>
    </AppShell>
  );
}
