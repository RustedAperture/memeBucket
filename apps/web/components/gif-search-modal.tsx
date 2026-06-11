"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { apiGet } from "@/lib/api";
import { Loader2 } from "lucide-react";

export function GifSearchModal({
  open,
  onOpenChange,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (url: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchGifs = useCallback(async (q: string) => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiGet<any>(`/api/gifs/search?q=${encodeURIComponent(q)}`);
      // Actual Klipy response format: { data: { current_page: 1, data: [ ... ] } }
      if (data && data.data && Array.isArray(data.data.data)) {
        setResults(data.data.data);
      } else if (data && Array.isArray(data.data)) {
        setResults(data.data);
      } else if (data && Array.isArray(data.results)) {
        setResults(data.results);
      } else {
        setResults([]);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load GIFs. Is the API key configured?");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      fetchGifs(query);
    }
  }, [open, fetchGifs, query]);

  function getImageUrl(result: any): string {
    // New Klipy structure
    if (result.file?.hd?.gif?.url) {
      return result.file.hd.gif.url;
    }
    if (result.file?.md?.gif?.url) {
      return result.file.md.gif.url;
    }
    // Old Klipy structure
    if (result.media?.gif?.url) {
      return result.media.gif.url;
    }
    // Giphy fallback
    if (result.images?.original?.url) {
      return result.images.original.url;
    }
    return result.url;
  }

  function getPreviewUrl(result: any): string {
    // New Klipy structure
    if (result.file?.sm?.gif?.url) {
      return result.file.sm.gif.url;
    }
    if (result.file?.xs?.gif?.url) {
      return result.file.xs.gif.url;
    }
    // Old Klipy structure
    if (result.media?.nanogif?.url) {
      return result.media.nanogif.url;
    }
    if (result.media?.preview?.url) {
      return result.media.preview.url;
    }
    // Giphy fallback
    if (result.images?.fixed_height_small?.url) {
      return result.images.fixed_height_small.url;
    }
    return getImageUrl(result);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Search GIFs</DialogTitle>
        </DialogHeader>
        <div className="pt-2 pb-4">
          <Input
            placeholder="Search GIFs..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div className="flex-1 overflow-y-auto">
          {error ? (
            <div className="text-center text-sm text-destructive py-8">{error}</div>
          ) : loading && results.length === 0 ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : results.length === 0 ? (
            <div className="text-center text-sm text-muted-foreground py-8">No results found</div>
          ) : (
            <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
              {results.map((result, idx) => {
                const imgUrl = getImageUrl(result);
                const previewUrl = getPreviewUrl(result);
                if (!imgUrl) return null;
                return (
                  <div
                    key={result.id || idx}
                    className="relative cursor-pointer group rounded-md overflow-hidden bg-muted aspect-video"
                    onClick={() => onSelect(imgUrl)}
                  >
                    <img
                      src={previewUrl}
                      alt="GIF preview"
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform"
                    />
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
