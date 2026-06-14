"use client";

import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import type { GifSearchSelection } from "@/lib/types";
import { Plus, Search, X } from "lucide-react";
import { GifSearchModal } from "./gif-search-modal";
import { ButtonGroup } from "@/components/ui/button-group";

export function ImageForm({ poolId, onCreated }: { poolId: string; onCreated: () => void }) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [selectedGif, setSelectedGif] = useState<GifSearchSelection | null>(null);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [isCreating, setIsCreating] = useState(false);
  const creatingRef = useRef(false);

  async function createImage(payload: { url: string; title?: string; tags?: string[] }) {
    if (creatingRef.current) {
      return false;
    }
    creatingRef.current = true;
    setIsCreating(true);
    try {
      await apiPost(`/api/pools/${poolId}/images`, payload);
      setUrl("");
      setSelectedGif(null);
      setSelectedTags([]);
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
    const shouldUseMetadata = selectedGif?.url === trimmedUrl;
    const payload = {
      url: trimmedUrl,
      ...(shouldUseMetadata && selectedGif.title ? { title: selectedGif.title } : {}),
      ...(shouldUseMetadata && selectedTags.length > 0 ? { tags: selectedTags } : {}),
    };
    try {
      await createImage(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not add image");
    }
  }

  function handleUrlChange(value: string) {
    setUrl(value);
    if (selectedGif && value.trim() !== selectedGif.url) {
      setSelectedGif(null);
      setSelectedTags([]);
    }
  }

  async function handleGifSelect(selection: GifSearchSelection, action: "add" | "stage") {
    setSearchOpen(false);
    setError(null);
    if (action === "add") {
      try {
        await createImage({
          url: selection.url,
          ...(selection.title ? { title: selection.title } : {}),
          ...(selection.tags.length > 0 ? { tags: selection.tags } : {}),
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : "Could not add image");
      }
      return;
    }

    setUrl(selection.url);
    setSelectedGif(selection);
    setSelectedTags(selection.tags);
  }

  function removeTag(tag: string) {
    setSelectedTags((current) => current.filter((value) => value !== tag));
  }

  return (
    <>
    <div className="relative">
      <form onSubmit={submit} className="flex items-center gap-2">
          <Input
            value={url}
            onChange={(event) => handleUrlChange(event.target.value)}
            placeholder="Paste URL..."
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
            onClick={() => setSearchOpen(true)}
            title="Search GIFs"
            disabled={isCreating}
          >
            <Search className="w-4 h-4" />
          </Button>
        </ButtonGroup>
      </form>
      {error ? <p className="absolute top-full mt-1 right-0 z-40 text-xs font-medium text-destructive whitespace-nowrap">{error}</p> : null}
      {selectedGif && (selectedGif.title || selectedTags.length > 0) ? (
        <div className="absolute right-0 top-full z-30 mt-2 w-80 rounded-md border bg-popover p-3 text-popover-foreground shadow-md">
          {selectedGif.title ? (
            <div className="mb-2 min-w-0 truncate text-sm font-medium">{selectedGif.title}</div>
          ) : null}
          {selectedTags.length > 0 ? (
            <div className="flex flex-wrap gap-1.5">
              {selectedTags.map((tag) => (
                <button
                  key={tag}
                  type="button"
                  onClick={() => removeTag(tag)}
                  className="inline-flex h-6 max-w-full items-center gap-1 rounded-md border bg-muted px-2 text-xs font-medium text-muted-foreground hover:text-foreground"
                  title={`Remove ${tag}`}
                >
                  <span className="truncate">{tag}</span>
                  <X className="h-3 w-3 shrink-0" />
                </button>
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
    
    <GifSearchModal 
      open={searchOpen} 
      onOpenChange={setSearchOpen} 
      onSelect={handleGifSelect} 
      disabled={isCreating}
    />
  </>
  );
}
