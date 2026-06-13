"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiGet } from "@/lib/api";
import type { GifSearchSelection } from "@/lib/types";
import { Loader2 } from "lucide-react";

type GifAsset = {
  url?: string;
};

type GifResult = {
  id?: string | number;
  url?: string;
  title?: string;
  slug?: string;
  tags?: unknown;
  file?: {
    hd?: { gif?: GifAsset };
    md?: { gif?: GifAsset };
    sm?: { gif?: GifAsset };
    xs?: { gif?: GifAsset };
  };
  media?: {
    gif?: GifAsset;
    nanogif?: GifAsset;
    preview?: GifAsset;
  };
  images?: {
    original?: GifAsset;
    fixed_height_small?: GifAsset;
  };
};

type GifSearchResponse = {
  data?: GifResult[] | { data?: GifResult[]; per_page?: number };
  results?: GifResult[];
};

const GIFS_PER_PAGE = 50;

export function GifSearchModal({
  open,
  onOpenChange,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (selection: GifSearchSelection) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<GifResult[]>([]);
  const resultsRef = useRef<GifResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    resultsRef.current = results;
  }, [results]);

  const fetchGifs = useCallback(async (q: string, pageToLoad = 1, append = false) => {
    if (append) {
      setLoadingMore(true);
    } else {
      setLoading(true);
    }
    setError(null);
    try {
      const data = await apiGet<GifSearchResponse>(
        `/api/gifs/search?q=${encodeURIComponent(q)}&page=${pageToLoad}&per_page=${GIFS_PER_PAGE}`
      );
      let nextResults: GifResult[] = [];
      let perPage = GIFS_PER_PAGE;

      // Actual Klipy response format: { data: { current_page: 1, data: [ ... ] } }
      if (data && data.data && !Array.isArray(data.data) && Array.isArray(data.data.data)) {
        nextResults = data.data.data;
        perPage = data.data.per_page || GIFS_PER_PAGE;
      } else if (data && Array.isArray(data.data)) {
        nextResults = data.data;
      } else if (data && Array.isArray(data.results)) {
        nextResults = data.results;
      }

      const current = append ? resultsRef.current : [];
      const existingIds = new Set(current.map((r) => r.id));
      const additions = nextResults.filter((r) => !existingIds.has(r.id));

      setResults(append ? [...current, ...additions] : nextResults);
      setPage(pageToLoad);
      // If we are appending and got no new additions, we've reached the end (or API is ignoring pagination)
      if (append && additions.length === 0) {
        setHasMore(false);
      } else {
        setHasMore(nextResults.length >= perPage);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load GIFs. Is the API key configured?");
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      const timeoutId = window.setTimeout(() => {
        void fetchGifs(query);
      }, 300);

      return () => window.clearTimeout(timeoutId);
    }
  }, [open, fetchGifs, query]);

  function handleLoadMore() {
    void fetchGifs(query, page + 1, true);
  }

  function getImageUrl(result: GifResult): string | undefined {
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

  function getPreviewUrl(result: GifResult): string | undefined {
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

  function buildSelection(result: GifResult, imageUrl: string): GifSearchSelection {
    return {
      url: imageUrl,
      title: normalizeTitle(result.title),
      slug: normalizeTitle(result.slug),
      tags: buildSuggestedTags(result, query),
    };
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Search GIFs</DialogTitle>
        </DialogHeader>
        <div className="pt-2 pb-4">
          <Input
            placeholder="Search KLIPY"
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
            <div className="flex flex-col gap-4">
              <div className="columns-2 md:columns-3 gap-2 space-y-2">
                {results.map((result, idx) => {
                  const imgUrl = getImageUrl(result);
                  const previewUrl = getPreviewUrl(result);
                  const title = normalizeTitle(result.title);
                  if (!imgUrl) return null;
                  return (
                    <div
                      key={result.id || idx}
                      className="relative cursor-pointer group rounded-md overflow-hidden bg-muted break-inside-avoid"
                      onClick={() => onSelect(buildSelection(result, imgUrl))}
                    >
                      <img
                        src={previewUrl}
                        alt={title || "GIF preview"}
                        className="w-full h-auto object-cover transition-transform group-hover:scale-105"
                      />
                    </div>
                  );
                })}
              </div>
              {hasMore ? (
                <Button
                  type="button"
                  variant="secondary"
                  onClick={handleLoadMore}
                  disabled={loadingMore}
                  className="self-center"
                >
                  {loadingMore ? (
                    <Loader2 className="animate-spin" />
                  ) : null}
                  Load more
                </Button>
              ) : null}
            </div>
          )}
        </div>
        <div className="border-t pt-3 text-center text-xs font-medium text-muted-foreground">
          Powered by KLIPY
        </div>
      </DialogContent>
    </Dialog>
  );
}

const STOP_WORDS = new Set([
  "a",
  "an",
  "and",
  "are",
  "as",
  "at",
  "be",
  "by",
  "for",
  "from",
  "gif",
  "gifs",
  "in",
  "is",
  "it",
  "its",
  "of",
  "on",
  "or",
  "sticker",
  "stickers",
  "that",
  "the",
  "this",
  "to",
  "with",
  "you",
  "your",
]);

function normalizeTitle(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.replace(/\s+/g, " ").trim();
  if (!normalized) {
    return null;
  }
  return normalized.slice(0, 200);
}

function buildSuggestedTags(result: GifResult, query: string): string[] {
  const tags: string[] = [];
  const seen = new Set<string>();

  const add = (value: unknown) => {
    for (const token of tagTokens(value)) {
      if (seen.has(token)) {
        continue;
      }
      seen.add(token);
      tags.push(token);
      if (tags.length >= 12) {
        return;
      }
    }
  };

  add(query);
  add(result.tags);
  add(result.title);
  add(result.slug);

  return tags;
}

function tagTokens(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.flatMap((item) => tagTokens(item));
  }

  if (value && typeof value === "object") {
    const tagged = value as { name?: unknown; title?: unknown; slug?: unknown };
    return [...tagTokens(tagged.name), ...tagTokens(tagged.title), ...tagTokens(tagged.slug)];
  }

  if (typeof value !== "string") {
    return [];
  }

  return value
    .toLowerCase()
    .replace(/&/g, " and ")
    .split(/[^a-z0-9_-]+/g)
    .map((token) => token.replace(/^[-_]+|[-_]+$/g, ""))
    .filter((token) => token.length >= 2 && !STOP_WORDS.has(token))
    .map((token) => token.slice(0, 32));
}
