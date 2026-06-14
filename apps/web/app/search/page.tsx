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
import {
  SidebarProvider,
  Sidebar,
  SidebarContent,
  SidebarInset,
  SidebarTrigger,
  SidebarHeader,
} from "@/components/ui/sidebar";
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
    <SidebarProvider className="h-full flex flex-1 min-h-0 w-full overflow-hidden rounded-xl bg-muted/30 border relative">
      {/* Sidebar Area for Filters */}
      <Sidebar className="absolute h-full bg-transparent border-r-0 hidden md:flex" collapsible="offcanvas" variant="inset">
        <SidebarHeader className="p-4 pb-2 border-b-0">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight">Library</h1>
            <p className="text-xs text-muted-foreground leading-relaxed">
              Search GIFs and images across all your saved pools.
            </p>
          </div>
        </SidebarHeader>
        <SidebarContent className="px-4 pt-2 pb-6 space-y-6 overflow-y-auto">
          <div className="space-y-1.5">
            <Label htmlFor="global-search-query" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Search</Label>
            <div className="relative min-w-0 flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                id="global-search-query"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Title, notes, pool, URL..."
                className="h-9 pl-9 text-sm bg-background"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="search-tags" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Tags</Label>
            <Input
              id="search-tags"
              value={tags}
              onChange={(event) => setTags(event.target.value)}
              placeholder="cat, reaction"
              className="h-9 text-sm bg-background"
            />
          </div>

          <div className="space-y-1.5">
            <Label id="search-pool-label" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Pool</Label>
            <Select
              items={poolItems}
              value={poolId}
              onValueChange={(value) => {
                if (typeof value === "string") {
                  setPoolId(value);
                }
              }}
            >
              <SelectTrigger className="w-full h-9 text-sm bg-background" aria-labelledby="search-pool-label">
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
              <Label id="search-random-label" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Random</Label>
              <Tooltip>
                <TooltipTrigger className="cursor-help outline-none p-0 bg-transparent border-none inline-flex items-center justify-center">
                  <HelpCircle className="h-3.5 w-3.5 text-muted-foreground hover:text-foreground transition-colors" />
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
              <SelectTrigger className="w-full h-9 text-sm bg-background" aria-labelledby="search-random-label">
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

          <div className="flex items-center justify-between rounded-md border bg-background px-3 py-2.5 shadow-sm">
            <Label htmlFor="favorite-only" className="flex items-center gap-2 cursor-pointer text-sm font-medium">
              <Star className={favoriteOnly ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
              Favorites Only
            </Label>
            <Switch
              id="favorite-only"
              checked={favoriteOnly}
              onCheckedChange={setFavoriteOnly}
              className="scale-90"
            />
          </div>

          {activeFilterCount > 0 ? (
            <div className="pt-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="w-full bg-background"
                onClick={() => {
                  setTags("");
                  setFavoriteOnly(false);
                  setRandomFilter("any");
                  setPoolId("all");
                }}
              >
                <X className="h-4 w-4 mr-2" />
                Clear Filters
              </Button>
            </div>
          ) : null}
        </SidebarContent>
      </Sidebar>

      {/* Inset Main Content Area for Results */}
      <SidebarInset className="flex-1 flex flex-col m-2 rounded-xl bg-background shadow-sm border overflow-hidden">
        <header className="flex h-14 shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear px-4 lg:px-6">
          <SidebarTrigger className="h-8 w-8 -ml-2 text-muted-foreground" />
          <div className="flex flex-1 items-center justify-between text-sm">
            <span className="font-semibold text-foreground">Search Results</span>
            <span className="text-muted-foreground">{loading ? "Searching..." : `${results.length} result${results.length === 1 ? "" : "s"}`}</span>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto p-4 sm:p-6 bg-muted/10">
          {poolError ? (
            <p className="text-sm font-medium text-destructive mb-4">{poolError}</p>
          ) : null}
          {searchError ? (
            <p className="text-sm font-medium text-destructive mb-4">{searchError}</p>
          ) : null}

          {loading ? (
            <div className="columns-1 sm:columns-2 lg:columns-3 gap-4">
              {Array.from({ length: 6 }).map((_, index) => (
                <div key={index} className="h-64 animate-pulse rounded-lg border bg-muted/40 break-inside-avoid mb-4" />
              ))}
            </div>
          ) : results.length === 0 ? (
            <div className="flex min-h-[60vh] flex-col items-center justify-center rounded-lg border border-dashed bg-background/50 p-8 text-center">
              <ImageIcon className="h-10 w-10 text-muted-foreground/50" />
              <h2 className="mt-4 text-lg font-semibold">No images found</h2>
              <p className="text-sm text-muted-foreground mt-1 max-w-sm">Try adjusting your filters or search terms.</p>
            </div>
          ) : (
            <div className="columns-1 sm:columns-2 lg:columns-3 gap-4">
              {results.map((result) => (
                <SearchResultCard key={result.image.id} result={result} />
              ))}
            </div>
          )}
        </div>
      </SidebarInset>
    </SidebarProvider>
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

        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            nativeButton={false}
            render={<Link href={`/pools?id=${result.poolId}`} />}
            className="min-w-0"
          >
            <FolderOpen className="h-4 w-4 shrink-0" />
            <span className="truncate">{result.poolName}</span>
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleCopy}
            className="shrink-0"
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
            className="shrink-0"
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
