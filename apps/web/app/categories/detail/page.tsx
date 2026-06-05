"use client";

import { AppShell } from "@/components/app-shell";
import { MediaLinkForm } from "@/components/media-link-form";

export default function CategoryDetailPage() {
  const params = new URLSearchParams(typeof window !== "undefined" ? window.location.search : "");
  const categoryId = params.get("id") ?? "";

  return (
    <AppShell>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold">Category</h1>
        {categoryId ? <MediaLinkForm categoryId={categoryId} onCreated={() => {}} /> : null}
      </div>
    </AppShell>
  );
}
