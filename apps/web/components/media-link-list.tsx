"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet } from "@/lib/api";

type MediaLinkItem = { id: string; url: string };

export function MediaLinkList({ categoryId }: { categoryId: string }) {
  const [links, setLinks] = useState<MediaLinkItem[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      setLinks(await apiGet<MediaLinkItem[]>(`/api/categories/${categoryId}/links`));
    } catch {
      // category might be empty or deleted
    }
  }

  useEffect(() => {
    void load();
  }, [categoryId]);

  async function handleDelete(linkId: string) {
    setError(null);
    try {
      await apiDelete(`/api/categories/${categoryId}/links/${linkId}`);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not delete link");
    }
  }

  if (links.length === 0) {
    return <p className="text-sm text-muted-foreground">No links in this category yet.</p>;
  }

  return (
    <div className="space-y-2">
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {links.map((link) => (
        <div key={link.id} className="flex items-center justify-between border-b py-2">
          <a href={link.url} target="_blank" rel="noreferrer" className="text-sm truncate max-w-md hover:underline">
            {link.url}
          </a>
          <Button variant="ghost" size="sm" onClick={() => handleDelete(link.id)}>
            Delete
          </Button>
        </div>
      ))}
    </div>
  );
}
