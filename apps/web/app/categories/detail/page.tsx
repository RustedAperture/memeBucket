"use client";

import { useEffect, useState } from "react";
import { AppShell } from "@/components/app-shell";
import { MediaLinkForm } from "@/components/media-link-form";
import { MediaLinkList } from "@/components/media-link-list";

export default function CategoryDetailPage() {
  const [categoryId, setCategoryId] = useState("");
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setCategoryId(params.get("id") ?? "");
  }, []);

  return (
    <AppShell>
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold">Category</h1>
        {categoryId ? (
          <>
            <MediaLinkForm
              categoryId={categoryId}
              onCreated={() => setRefreshKey((k) => k + 1)}
            />
            <MediaLinkList key={refreshKey} categoryId={categoryId} />
          </>
        ) : (
          <p className="text-muted-foreground">No category selected.</p>
        )}
      </div>
    </AppShell>
  );
}
