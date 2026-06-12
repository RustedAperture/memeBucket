"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import { Plus, Search } from "lucide-react";
import { GifSearchModal } from "./gif-search-modal";
import { ButtonGroup } from "@/components/ui/button-group";

export function ImageForm({ poolId, onCreated }: { poolId: string; onCreated: () => void }) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!url.trim()) return;
    setError(null);
    try {
      await apiPost(`/api/pools/${poolId}/images`, { url });
      setUrl("");
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not add image");
    }
  }

  async function handleGifSelect(selectedUrl: string) {
    setSearchOpen(false);
    setError(null);
    try {
      await apiPost(`/api/pools/${poolId}/images`, { url: selectedUrl });
      setUrl("");
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not add image");
    }
  }

  return (
    <>
    <form onSubmit={submit} className="flex items-center gap-2 relative">
      <Input 
        value={url} 
        onChange={(event) => setUrl(event.target.value)} 
        placeholder="Paste URL..." 
        className="h-8 w-48 text-sm"
      />
      <ButtonGroup>
        <Button 
          type="submit" 
          variant="default"
          size="icon" 
          title="Add Image"
        >
          <Plus className="w-4 h-4" />
        </Button>
        <Button
          type="button"
          variant="default"
          size="icon"
          onClick={() => setSearchOpen(true)}
          title="Search GIFs"
        >
          <Search className="w-4 h-4" />
        </Button>
      </ButtonGroup>
      {error ? <p className="absolute top-full mt-1 right-0 text-xs font-medium text-destructive whitespace-nowrap">{error}</p> : null}
    </form>
    
    <GifSearchModal 
      open={searchOpen} 
      onOpenChange={setSearchOpen} 
      onSelect={handleGifSelect} 
    />
  </>
  );
}
