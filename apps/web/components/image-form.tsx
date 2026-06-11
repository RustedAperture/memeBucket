"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import { Plus, Search } from "lucide-react";
import { GifSearchModal } from "./gif-search-modal";

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
      <form onSubmit={submit} className="flex flex-col gap-4">
      <Input 
        value={url} 
        onChange={(event) => setUrl(event.target.value)} 
        placeholder="https://example.com/image.gif" 
        className="w-full"
      />
      <div className="flex justify-between items-center">
        <Button
          type="button"
          variant="secondary"
          onClick={() => setSearchOpen(true)}
        >
          <Search className="w-4 h-4 mr-2" />
          Search GIFs
        </Button>
        <Button type="submit">
          <Plus className="w-4 h-4 mr-2" />
          Add Image
        </Button>
      </div>
      {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
    </form>
    
    <GifSearchModal 
      open={searchOpen} 
      onOpenChange={setSearchOpen} 
      onSelect={handleGifSelect} 
    />
  </>
  );
}
