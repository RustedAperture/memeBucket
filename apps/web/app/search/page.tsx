"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  Check,
  Copy,
  ExternalLink,
  FolderOpen,
  HelpCircle,
  ImageIcon,
  Search,
  Star,
  X,
} from "lucide-react";
import { AppShell } from "@/components/app-shell";
import { RequireAuth } from "@/components/require-auth";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { apiGet } from "@/lib/api";
import type { ImageSearchResult, Pool } from "@/lib/types";
import { toast } from "sonner";

type RandomFilter = "any" | "enabled" | "disabled";

export default function SearchPage() {
  return (
    <AppShell>
      <RequireAuth>
        <SearchContent />
      </RequireAuth>
    </AppShell>
  );
}

function SearchContent() {
  const [query, setQuery] = useState("");
  const [tags, setTags] = useState("");
  const [favoriteOnly, setFavoriteOnly] = useState(false);
  const [randomFilter, setRandomFilter] = useState<RandomFilter>("any");
  const [poolId, setPoolId] = useState("all");
  const [pools, setPools] = useState<Pool[]>([]);
  const [results, setResults] = useState<ImageSearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [poolError, setPoolError] = useState<string | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);

  const poolItems = useMemo(
    () => [
      { label: "All pools", value: "all" },
      ...pools.map((pool) => ({
        label: `${pool.name}${pool.is_subscribed ? " (Subscribed)" : ""}`,
        value: pool.id,
      })),
    ],
    [pools]
  );

  useEffect(() => {
    let cancelled = false;
    void apiGet<Pool[]>("/api/pools")
      .then((loaded) => {
        if (!cancelled) {
          setPools(loaded);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setPoolError(err instanceof Error ? err.message : "Could not load pools");
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timeout = window.setTimeout(() => {
      setLoading(true);
      setSearchError(null);
      void apiGet<ImageSearchResult[]>(`/api/images/search?${searchParams({
        query,
        tags,
        favoriteOnly,
        randomFilter,
        poolId,
      })}`)
        .then((loaded) => {
          if (!cancelled) {
            setResults(loaded);
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setSearchError(err instanceof Error ? err.message : "Could not search images");
            setResults([]);
          }
        })
        .finally(() => {
          if (!cancelled) {
            setLoading(false);
          }
        });
    }, 250);

    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
    };
  }, [query, tags, favoriteOnly, randomFilter, poolId]);

  const activeFilterCount = [
    tags.trim(),
    favoriteOnly,
    randomFilter !== "any",
    poolId !== "all",
  ].filter(Boolean).length;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="flex flex-col gap-3 border-b pb-4">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Library</h1>
          <p className="text-sm text-muted-foreground">
            Search GIFs and images already saved in your pools.
          </p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <div className="relative min-w-0 flex-1">
            <Label htmlFor="global-search-query" className="sr-only">
              Search saved images
            </Label>
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              id="global-search-query"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search saved title, tag, notes, pool, or URL"
              className="h-10 pl-9"
            />
          </div>
          {activeFilterCount > 0 ? (
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setTags("");
                setFavoriteOnly(false);
                setRandomFilter("any");
                setPoolId("all");
              }}
            >
              <X className="h-4 w-4" />
              Clear
            </Button>
          ) : null}
        </div>

        <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_12rem_11rem_auto]">
          <div className="space-y-1.5">
            <Label htmlFor="search-tags">Tags</Label>
            <Input
              id="search-tags"
              value={tags}
              onChange={(event) => setTags(event.target.value)}
              placeholder="cat, reaction"
            />
          </div>

          <div className="space-y-1.5">
            <Label id="search-pool-label">Pool</Label>
            <Select
              items={poolItems}
              value={poolId}
              onValueChange={(value) => {
                if (typeof value === "string") {
                  setPoolId(value);
                }
              }}
            >
              <SelectTrigger className="w-full" aria-labelledby="search-pool-label">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Pools</SelectLabel>
                  <SelectItem value="all">All pools</SelectItem>
                  {pools.map((pool) => (
                    <SelectItem key={pool.id} value={pool.id}>
                      {pool.name}{pool.is_subscribed ? " (Subscribed)" : ""}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <div className="flex items-center gap-1.5">
              <Label id="search-random-label">Random</Label>
              <Tooltip>
                <TooltipTrigger className="cursor-help outline-none p-0 bg-transparent border-none inline-flex items-center justify-center">
                  <HelpCircle className="h-4 w-4 text-muted-foreground hover:text-foreground transition-colors" />
                </TooltipTrigger>
                <TooltipContent className="max-w-xs">
                  <div className="flex flex-col gap-1.5">
                    <p>Filter by whether images can be randomly selected.</p>
                    <ul className="list-disc pl-4 opacity-90">
                      <li><strong>Enabled</strong>: Weight &gt; 0</li>
                      <li><strong>Disabled</strong>: Weight = 0</li>
                    </ul>
                  </div>
                </TooltipContent>
              </Tooltip>
            </div>
            <Select
              items={[
                { label: "Any", value: "any" },
                { label: "Enabled", value: "enabled" },
                { label: "Disabled", value: "disabled" },
              ]}
              value={randomFilter}
              onValueChange={(value) => {
                if (value === "any" || value === "enabled" || value === "disabled") {
                  setRandomFilter(value);
                }
              }}
            >
              <SelectTrigger className="w-full" aria-labelledby="search-random-label">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="any">Any</SelectItem>
                  <SelectItem value="enabled">Enabled</SelectItem>
                  <SelectItem value="disabled">Disabled</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <Label id="favorite-label" className="invisible">Favorites</Label>
            <div className="flex h-10 items-center gap-2 rounded-md border px-3">
              <Switch
                id="favorite-only"
                checked={favoriteOnly}
                onCheckedChange={setFavoriteOnly}
                aria-labelledby="favorite-label"
              />
              <Label htmlFor="favorite-only" className="cursor-pointer whitespace-nowrap">
                <Star className={favoriteOnly ? "h-4 w-4 fill-current text-primary -mt-0.5" : "h-4 w-4 text-muted-foreground -mt-0.5"} />
                Favorites
              </Label>
            </div>
          </div>
        </div>
      </div>

      {poolError ? (
        <p className="text-sm font-medium text-destructive">{poolError}</p>
      ) : null}
      {searchError ? (
        <p className="text-sm font-medium text-destructive">{searchError}</p>
      ) : null}

      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <span>{loading ? "Searching..." : `${results.length} result${results.length === 1 ? "" : "s"}`}</span>
      </div>

      {loading ? (
        <div className="columns-1 sm:columns-2 lg:columns-3 gap-4">
          {Array.from({ length: 6 }).map((_, index) => (
            <div key={index} className="h-64 animate-pulse rounded-lg border bg-muted/40 break-inside-avoid mb-4" />
          ))}
        </div>
      ) : results.length === 0 ? (
        <div className="flex min-h-64 flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center">
          <ImageIcon className="h-10 w-10 text-muted-foreground/50" />
          <h2 className="mt-4 text-lg font-semibold">No images found</h2>
        </div>
      ) : (
        <div className="columns-1 sm:columns-2 lg:columns-3 gap-4">
          {results.map((result) => (
            <SearchResultCard key={result.image.id} result={result} />
          ))}
        </div>
      )}
    </div>
  );
}

function SearchResultCard({ result }: { result: ImageSearchResult }) {
  const image = result.image;
  const isVideo = isVideoUrl(image.url);
  const [mediaFailed, setMediaFailed] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(image.url);
      setCopied(true);
      toast.success("Link copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      toast.error("Failed to copy link");
    }
  };

  return (
    <article className="min-w-0 overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm break-inside-avoid mb-4">
      <div className="relative bg-muted">
        {mediaFailed ? (
          <div className="flex aspect-[4/3] w-full items-center justify-center">
            <ImageIcon className="h-10 w-10 text-muted-foreground/50" />
          </div>
        ) : isVideo ? (
          <video
            src={image.url}
            autoPlay
            loop
            muted
            playsInline
            className="w-full h-auto block"
            onError={() => setMediaFailed(true)}
          />
        ) : (
          <img
            src={image.url}
            alt={image.title || "Image preview"}
            className="w-full h-auto block"
            onError={() => setMediaFailed(true)}
          />
        )}
      </div>
      <div className="space-y-3 p-3">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold">{image.title || image.url}</h2>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {image.favorite ? (
            <Badge variant="secondary" className="rounded-md">
              <Star className="h-3 w-3 fill-current" />
              Favorite
            </Badge>
          ) : null}
          <Badge variant="outline" className="rounded-md">Weight {image.randomWeight}</Badge>
          <Badge variant="outline" className="rounded-md">{image.sendCount} send{image.sendCount === 1 ? "" : "s"}</Badge>
        </div>

        {image.tags.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {image.tags.slice(0, 6).map((tag) => (
              <Badge key={tag} variant="secondary" className="max-w-full rounded-md">
                <span className="truncate">{tag}</span>
              </Badge>
            ))}
            {image.tags.length > 6 ? (
              <Badge variant="outline" className="rounded-md">+{image.tags.length - 6}</Badge>
            ) : null}
          </div>
        ) : null}

        <div className="flex gap-2">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            nativeButton={false}
            render={<Link href={`/pools?id=${result.poolId}`} />}
            className="max-w-full"
          >
            <FolderOpen className="h-4 w-4 shrink-0" />
            <span className="truncate">{result.poolName}</span>
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleCopy}
          >
            {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
            Copy
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            nativeButton={false}
            render={<a href={image.url} target="_blank" rel="noreferrer" />}
          >
            <ExternalLink className="h-4 w-4" />
            Open
          </Button>
        </div>
      </div>
    </article>
  );
}

function searchParams({
  query,
  tags,
  favoriteOnly,
  randomFilter,
  poolId,
}: {
  query: string;
  tags: string;
  favoriteOnly: boolean;
  randomFilter: RandomFilter;
  poolId: string;
}) {
  const params = new URLSearchParams();
  params.set("limit", "60");

  if (query.trim()) {
    params.set("q", query.trim());
  }
  if (tags.trim()) {
    params.set("tags", tags.trim());
  }
  if (favoriteOnly) {
    params.set("favorite", "true");
  }
  if (randomFilter !== "any") {
    params.set("randomEnabled", String(randomFilter === "enabled"));
  }
  if (poolId !== "all") {
    params.set("poolId", poolId);
  }

  return params.toString();
}

function isVideoUrl(url: string) {
  const base = url.split("?")[0].toLowerCase();
  return base.endsWith(".mp4") || base.endsWith(".webm");
}
