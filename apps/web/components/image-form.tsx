"use client";

import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import type { GifSearchSelection } from "@/lib/types";
import { Plus, Search, X } from "lucide-react";
import { GifSearchModal } from "./gif-search-modal";
import { ButtonGroup } from "@/components/ui/button-group";

export function ImageForm({ bucketId, onCreated }: { bucketId: string; onCreated: () => void }) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const creatingRef = useRef(false);

  async function createImage(payload: { url: string; title?: string; tags?: string[] }) {
    if (creatingRef.current) {
      return false;
    }
    creatingRef.current = true;
    setIsCreating(true);
    try {
      await apiPost(`/api/buckets/${bucketId}/images`, payload);
      setUrl("");
      onCreated();
      return true;
    } finally {
      creatingRef.current = false;
      setIsCreating(false);
    }
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const trimmedUrl = url.trim();
    if (!trimmedUrl) return;
    setError(null);

    // If it doesn't look like a URL, open search
    try {
      new URL(trimmedUrl);
    } catch {
      setSearchQuery(trimmedUrl);
      setSearchOpen(true);
      return;
    }

    const payload = {
      url: trimmedUrl,
    };
    try {
      await createImage(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not add image");
    }
  }

  function handleUrlChange(value: string) {
    setUrl(value);
  }

  async function handleGifSelect(selection: GifSearchSelection) {
    setSearchOpen(false);
    setError(null);
    try {
      await createImage({
        url: selection.url,
        ...(selection.title ? { title: selection.title } : {}),
        ...(selection.tags.length > 0 ? { tags: selection.tags } : {}),
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not add image");
    }
  }

  return (
    <>
    <div className="relative">
      <form onSubmit={submit} className="flex items-center gap-2">
          <Input
            value={url}
            onChange={(event) => handleUrlChange(event.target.value)}
            placeholder="URL or search KLIPY..."
            className="h-8 w-48 text-sm"
            disabled={isCreating}
          />
        <ButtonGroup>
          <Button
            type="submit"
            variant="default"
            size="icon"
            title="Add Image"
            disabled={isCreating}
          >
            <Plus className="w-4 h-4" />
          </Button>
          <Button
            type="button"
            variant="default"
            size="icon"
            onClick={() => {
              setSearchQuery(url.trim());
              setSearchOpen(true);
            }}
            title="Search GIFs"
            disabled={isCreating}
          >
            <Search className="w-4 h-4" />
          </Button>
        </ButtonGroup>
      </form>
      {error ? <p className="absolute top-full mt-1 right-0 z-40 text-xs font-medium text-destructive whitespace-nowrap">{error}</p> : null}
    </div>
        <GifSearchModal
        open={searchOpen}
        onOpenChange={(open) => {
          setSearchOpen(open);
          if (!open) {
            setSearchQuery(""); // Clear the initial query so it doesn't persist forever
          }
        }}
        onSelect={handleGifSelect}
        initialQuery={searchQuery}
        disabled={isCreating}
      />
  </>
  );
}
