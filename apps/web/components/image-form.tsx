"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import { Plus } from "lucide-react";

export function ImageForm({ poolId, onCreated }: { poolId: string; onCreated: () => void }) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);

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

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Input 
        value={url} 
        onChange={(event) => setUrl(event.target.value)} 
        placeholder="https://example.com/image.gif" 
        className="w-full"
      />
      <div className="flex justify-end">
        <Button type="submit">
          <Plus className="w-4 h-4 mr-2" />
          Add Image
        </Button>
      </div>
      {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
    </form>
  );
}
