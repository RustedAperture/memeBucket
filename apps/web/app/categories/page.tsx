import { AppShell } from "@/components/app-shell";

export default function CategoriesPage() {
  return (
    <AppShell>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold">Categories</h1>
        <div id="category-list-root" />
      </div>
    </AppShell>
  );
}
