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
  Edit2,
  Trash2,
  Tags,
  Ban,
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
import { apiGet, apiPatch, apiDelete } from "@/lib/api";
import { useIsMobile } from "@/hooks/use-mobile";
import { useTouchHold } from "@/hooks/use-touch-hold";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import type { ImageSearchResult, Bucket, ImageItem } from "@/lib/types";
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
  const [bucketId, setBucketId] = useState("all");
  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [results, setResults] = useState<ImageSearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [bucketError, setBucketError] = useState<string | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [sizeIndex, setSizeIndex] = useState(3);

  const COLUMN_CLASSES = [
    "columns-3 sm:columns-4 md:columns-5 lg:columns-6",
    "columns-2 sm:columns-3 md:columns-4 lg:columns-5",
    "columns-2 sm:columns-2 md:columns-3 lg:columns-4",
    "columns-1 sm:columns-2 md:columns-2 lg:columns-3",
    "columns-1 sm:columns-1 md:columns-2 lg:columns-2",
  ];
  const SIZE_LABELS = ["-2", "-1", "0", "+1", "+2"];
  const columnClass = COLUMN_CLASSES[sizeIndex] || COLUMN_CLASSES[2];

  const bucketItems = useMemo(
    () => [
      { label: "All buckets", value: "all" },
      ...buckets.map((bucket) => ({
        label: `${bucket.name}${bucket.is_subscribed ? " (Subscribed)" : ""}`,
        value: bucket.id,
      })),
    ],
    [buckets]
  );

  useEffect(() => {
    let cancelled = false;
    void apiGet<Bucket[]>("/api/buckets")
      .then((loaded) => {
        if (!cancelled) {
          setBuckets(loaded);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setBucketError(err instanceof Error ? err.message : "Could not load buckets");
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
        bucketId,
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
  }, [query, tags, favoriteOnly, randomFilter, bucketId]);

  const activeFilterCount = [
    tags.trim(),
    favoriteOnly,
    randomFilter !== "any",
    bucketId !== "all",
  ].filter(Boolean).length;

  return (
    <SidebarProvider className="h-full flex flex-1 min-h-0 w-full overflow-hidden rounded-xl bg-muted/30 border relative">
      {/* Sidebar Area for Filters */}
      <Sidebar className="absolute h-full bg-transparent border-r-0 hidden md:flex" collapsible="offcanvas" variant="inset">
        <SidebarHeader className="p-4 pb-2 border-b-0">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight">Library</h1>
            <p className="text-xs text-muted-foreground leading-relaxed">
              Search GIFs and images across all your saved buckets.
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
                placeholder="Title, notes, bucket, URL..."
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
            <Label id="search-bucket-label" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Bucket</Label>
            <Select
              items={bucketItems}
              value={bucketId}
              onValueChange={(value) => {
                if (typeof value === "string") {
                  setBucketId(value);
                }
              }}
            >
              <SelectTrigger className="w-full h-9 text-sm bg-background" aria-labelledby="search-bucket-label">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Buckets</SelectLabel>
                  <SelectItem value="all">All buckets</SelectItem>
                  {buckets.map((bucket) => (
                    <SelectItem key={bucket.id} value={bucket.id}>
                      {bucket.name}{bucket.is_subscribed ? " (Subscribed)" : ""}
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
                  setBucketId("all");
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

        <div className="flex items-center gap-3 px-4 lg:px-6 py-2 sm:py-0 h-auto sm:h-12 shrink-0 border-b bg-muted/30 w-full">
          <span className="text-xs font-medium text-muted-foreground whitespace-nowrap">
            Size
          </span>
          <div className="relative flex items-center w-24 h-6">
            <div className="absolute left-0 right-0 h-0.5 rounded-full bg-border" />
            <div
              className="absolute left-0 h-0.5 rounded-full bg-primary transition-all duration-150"
              style={{ width: `${(sizeIndex / (COLUMN_CLASSES.length - 1)) * 100}%` }}
            />
            {COLUMN_CLASSES.map((_, i) => (
              <button
                key={i}
                type="button"
                onClick={() => setSizeIndex(i)}
                className="absolute flex items-center justify-center"
                style={{ left: `${(i / (COLUMN_CLASSES.length - 1)) * 100}%`, transform: "translateX(-50%)" }}
                aria-label={`Thumbnail size ${SIZE_LABELS[i]}`}
              >
                <span
                  className={`block rounded-full border-2 transition-all duration-150 ${
                    i === sizeIndex
                      ? "h-3.5 w-3.5 border-primary bg-primary shadow-sm"
                      : "h-2.5 w-2.5 border-muted-foreground/40 bg-background hover:border-primary/60"
                  }`}
                />
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4 sm:p-6 bg-muted/10">
          {bucketError ? (
            <p className="text-sm font-medium text-destructive mb-4">{bucketError}</p>
          ) : null}
          {searchError ? (
            <p className="text-sm font-medium text-destructive mb-4">{searchError}</p>
          ) : null}

          {loading ? (
            <div className={`${columnClass} gap-4`}>
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
            <div className={`${columnClass} gap-4`}>
              {results.map((result) => {
                const bucket = buckets.find((b) => b.id === result.bucketId);
                const readonly = bucket ? (bucket.is_subscribed || bucket.is_read_only) : true;
                return (
                  <SearchResultCard
                    key={result.image.id}
                    result={result}
                    readonly={readonly}
                    buckets={buckets}
                    onDelete={(imageId) => {
                      setResults((prev) => prev.filter((r) => r.image.id !== imageId));
                    }}
                  />
                );
              })}
            </div>
          )}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}

interface SearchResultCardProps {
  result: ImageSearchResult;
  readonly: boolean;
  buckets: Bucket[];
  onDelete: (imageId: string) => void;
}

function SearchResultCard({ result, readonly, buckets, onDelete }: SearchResultCardProps) {
  const isMobile = useIsMobile();
  const [image, setImage] = useState<ImageItem>(result.image);
  const [currentBucketId, setCurrentBucketId] = useState(result.bucketId);
  const [currentBucketName, setCurrentBucketName] = useState(result.bucketName);

  const isVideo = isVideoUrl(image.url);
  const [mediaFailed, setMediaFailed] = useState(false);
  const [copied, setCopied] = useState(false);

  // Details dialog states
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [editingMetadata, setEditingMetadata] = useState(false);
  const [titleValue, setTitleValue] = useState("");
  const [favoriteValue, setFavoriteValue] = useState(false);
  const [randomWeightValue, setRandomWeightValue] = useState(1);
  const [tagsValue, setTagsValue] = useState("");
  const [notesValue, setNotesValue] = useState("");
  const [imageToDelete, setImageToDelete] = useState<string | null>(null);

  const openImageDetails = () => {
    setTitleValue(image.title || "");
    setFavoriteValue(image.favorite);
    setRandomWeightValue(image.randomWeight);
    setTagsValue(image.tags.join(", "));
    setNotesValue(image.notes || "");
    setEditingMetadata(false);
    setDetailsOpen(true);
  };

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

  const touchHandlers = useTouchHold({
    onTap: handleCopy,
    onLongPress: openImageDetails,
  });

  const handleSaveMetadata = async () => {
    const normalizedTags = parseTagInput(tagsValue);
    const normalizedTitle = titleValue.trim() || null;
    const normalizedNotes = notesValue.trim() || null;
    const normalizedWeight = clampRandomWeight(randomWeightValue);
    try {
      await apiPatch(`/api/buckets/${currentBucketId}/images/${image.id}`, {
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      });
      const updatedImage = {
        ...image,
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      };
      setImage(updatedImage);
      setEditingMetadata(false);
      toast.success("Image details saved");
    } catch {
      toast.error("Failed to save image details");
    }
  };

  const handleMoveToBucket = async (newBucketId: string) => {
    try {
      await apiPatch(`/api/buckets/${currentBucketId}/images/${image.id}/move`, {
        destinationBucketId: newBucketId,
      });
      const destBucket = buckets.find((b) => b.id === newBucketId);
      if (destBucket) {
        setCurrentBucketId(newBucketId);
        setCurrentBucketName(destBucket.name);
      }
      toast.success("Image moved successfully");
      setDetailsOpen(false);
    } catch {
      toast.error("Failed to move image");
    }
  };

  const handleDeleteConfirm = async () => {
    if (!imageToDelete) return;
    try {
      await apiDelete(`/api/buckets/${currentBucketId}/images/${imageToDelete}`);
      toast.success("Image deleted successfully");
      setImageToDelete(null);
      setDetailsOpen(false);
      onDelete(image.id);
    } catch {
      toast.error("Failed to delete image");
    }
  };

  const moveBucketItems = buckets.map((p) => ({
    label: p.name,
    value: p.id,
  }));

  return (
    <>
      <article className="min-w-0 overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm break-inside-avoid mb-4">
        <div 
          className="relative bg-muted cursor-pointer group overflow-hidden rounded-t-lg"
          onClick={() => {
            if (isMobile) return;
            openImageDetails();
          }}
        >
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
              className="w-full h-auto block transition-transform duration-300 group-hover:scale-[1.02] select-none"
              style={{ WebkitTouchCallout: "none" }}
              onError={() => setMediaFailed(true)}
              onContextMenu={(e) => isMobile && e.preventDefault()}
              {...(isMobile ? touchHandlers : {})}
            />
          ) : (
            <img
              src={image.url}
              alt={image.title || "Image preview"}
              className="w-full h-auto block transition-transform duration-300 group-hover:scale-[1.02] select-none"
              style={{ WebkitTouchCallout: "none" }}
              onError={() => setMediaFailed(true)}
              onContextMenu={(e) => isMobile && e.preventDefault()}
              {...(isMobile ? touchHandlers : {})}
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
              render={<Link href={`/buckets?id=${currentBucketId}`} />}
              className="min-w-0"
            >
              <FolderOpen className="h-4 w-4 shrink-0" />
              <span className="truncate">{currentBucketName}</span>
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

      <Dialog open={detailsOpen} onOpenChange={setDetailsOpen}>
        <DialogContent className="min-w-0 max-h-[calc(100dvh-2rem)] grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle className="truncate">{image.title || "Image details"}</DialogTitle>
            <DialogDescription>
              {formatAddedAt(image.createdAt)} - {image.sendCount} send{image.sendCount === 1 ? "" : "s"}
            </DialogDescription>
          </DialogHeader>

          <div className="grid min-w-0 min-h-0 grid-rows-[minmax(0,1fr)_auto_auto_auto] gap-4 overflow-y-auto pr-1">
            <div className="min-h-0 overflow-hidden rounded-xl border border-border/70 bg-muted/20">
              {isVideo ? (
                <video
                  src={image.url}
                  autoPlay
                  loop
                  muted
                  playsInline
                  className="h-full max-h-full w-full object-contain"
                />
              ) : (
                <img
                  src={image.url}
                  alt={image.title || "Selected image preview"}
                  className="h-full max-h-full w-full object-contain"
                />
              )}
            </div>

            <div className="space-y-3 rounded-lg border border-border/70 p-3">
              <div className="flex items-center justify-between gap-3">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Metadata</p>
                {!readonly && !editingMetadata ? (
                  <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => setEditingMetadata(true)}>
                    <Edit2 className="h-3 w-3 mr-1" /> Edit
                  </Button>
                ) : null}
              </div>

              {editingMetadata ? (
                <div className="space-y-3">
                  <div className="space-y-1.5">
                    <Label htmlFor="image-title">Title</Label>
                    <Input
                      id="image-title"
                      value={titleValue}
                      maxLength={200}
                      onChange={(event) => setTitleValue(event.target.value)}
                      placeholder="Untitled"
                    />
                  </div>

                  <div className="grid gap-3 sm:grid-cols-[1fr_8rem]">
                    <div className="space-y-1.5">
                      <Label htmlFor="image-tags">Tags</Label>
                      <Input
                        id="image-tags"
                        value={tagsValue}
                        onChange={(event) => setTagsValue(event.target.value)}
                        placeholder="cat, reaction, happy"
                      />
                    </div>
                    <div className="space-y-1.5">
                      <div className="flex items-center gap-1.5">
                        <Label htmlFor="image-weight">Weight</Label>
                        <Tooltip>
                          <TooltipTrigger className="cursor-help outline-none p-0 bg-transparent border-none inline-flex items-center justify-center">
                            <HelpCircle className="h-4 w-4 text-muted-foreground hover:text-foreground transition-colors" />
                          </TooltipTrigger>
                          <TooltipContent className="max-w-xs">
                            <div className="flex flex-col gap-1.5">
                              <p>Weight (0-10) determines how likely this image is to be picked randomly.</p>
                              <ul className="list-disc pl-4 opacity-90">
                                <li><strong>0</strong>: Disabled (never picked)</li>
                                <li><strong>1-10</strong>: Higher = more likely</li>
                              </ul>
                            </div>
                          </TooltipContent>
                        </Tooltip>
                      </div>
                      <Input
                        id="image-weight"
                        type="number"
                        min={0}
                        max={10}
                        step={1}
                        value={randomWeightValue}
                        onChange={(event) => setRandomWeightValue(clampRandomWeight(Number(event.target.value)))}
                      />
                    </div>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-2">
                    <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
                      <Label htmlFor="image-favorite" className="flex items-center gap-2 cursor-pointer">
                        <Star className={favoriteValue ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
                        Favorite
                      </Label>
                      <Switch
                        id="image-favorite"
                        checked={favoriteValue}
                        onCheckedChange={setFavoriteValue}
                      />
                    </div>
                    
                    <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
                      <Label htmlFor="image-weight-disable" className="flex items-center gap-2 cursor-pointer">
                        <Ban className={randomWeightValue === 0 ? "h-4 w-4 text-destructive" : "h-4 w-4 text-muted-foreground"} />
                        Disable usage
                      </Label>
                      <Switch
                        id="image-weight-disable"
                        checked={randomWeightValue === 0}
                        onCheckedChange={(checked) => setRandomWeightValue(checked ? 0 : 1)}
                        className="data-[state=checked]:bg-destructive"
                      />
                    </div>
                  </div>

                  <div className="space-y-1.5">
                    <Label htmlFor="image-notes">Notes / Credits</Label>
                    <Textarea
                      id="image-notes"
                      value={notesValue}
                      onChange={(event) => setNotesValue(event.target.value)}
                      placeholder="Add notes, credits, or context..."
                      className="resize-none h-24"
                    />
                  </div>

                  <div className="flex justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setEditingMetadata(false);
                        setTitleValue(image.title || "");
                        setFavoriteValue(image.favorite);
                        setRandomWeightValue(image.randomWeight);
                        setTagsValue(image.tags.join(", "));
                        setNotesValue(image.notes || "");
                      }}
                    >
                      Cancel
                    </Button>
                    <Button size="sm" onClick={handleSaveMetadata}>Save</Button>
                  </div>
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="grid gap-2 text-sm sm:grid-cols-3">
                    <div>
                      <p className="text-xs text-muted-foreground">Favorite</p>
                      <p className="flex items-center gap-1 font-medium">
                        <Star className={image.favorite ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
                        {image.favorite ? "Yes" : "No"}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">Weight</p>
                      <p className="font-medium">{image.randomWeight}</p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">Sends</p>
                      <p className="font-medium">{image.sendCount}</p>
                    </div>
                  </div>
                  <div className="space-y-1.5">
                    <p className="text-xs text-muted-foreground">Tags</p>
                    {image.tags.length > 0 ? (
                      <div className="flex flex-wrap gap-1.5">
                        {image.tags.map((tag) => (
                          <Badge key={tag} variant="secondary" className="max-w-full rounded-md">
                            <span className="truncate">{tag}</span>
                          </Badge>
                        ))}
                      </div>
                    ) : (
                      <p className="text-sm text-muted-foreground">No tags</p>
                    )}
                  </div>
                  <div className="space-y-1.5">
                    <p className="text-xs text-muted-foreground">Notes / Credits</p>
                    <p className="min-h-5 whitespace-pre-wrap text-sm text-foreground">
                      {image.notes || "No notes provided."}
                    </p>
                  </div>
                </div>
              )}
            </div>

            <div className="space-y-2 min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Link</p>
              <div className="flex min-w-0 gap-2">
                <Input readOnly value={image.url} title={image.url} />
                <Button
                  variant="secondary"
                  size="icon"
                  aria-label="Open image link"
                  render={<a href={image.url} target="_blank" rel="noreferrer" />}
                >
                  <ExternalLink />
                </Button>
              </div>
            </div>

            {buckets.length > 1 && !readonly && (
              <div className="space-y-2">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Move to Bucket</p>
                <Select
                  items={moveBucketItems}
                  value={currentBucketId}
                  onValueChange={(newBucketId) => {
                    if (typeof newBucketId === "string") {
                      void handleMoveToBucket(newBucketId);
                    }
                  }}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectLabel>Buckets</SelectLabel>
                      {buckets.map((p) => (
                        <SelectItem
                          key={p.id}
                          value={p.id}
                          disabled={p.is_subscribed || p.id === currentBucketId}
                        >
                          {p.name}{p.id === currentBucketId ? " (Current)" : ""}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>
            )}
          </div>

          {!readonly && (
            <DialogFooter>
              <Button
                variant="destructive"
                onClick={() => setImageToDelete(image.id)}
              >
                <Trash2 className="w-4 h-4 mr-2" />
                Delete image
              </Button>
            </DialogFooter>
          )}
        </DialogContent>
      </Dialog>

      <AlertDialog open={!!imageToDelete} onOpenChange={(open) => !open && setImageToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you sure?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete the image. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteConfirm}
              className="bg-destructive hover:bg-destructive/90 text-destructive-foreground"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

function searchParams({
  query,
  tags,
  favoriteOnly,
  randomFilter,
  bucketId,
}: {
  query: string;
  tags: string;
  favoriteOnly: boolean;
  randomFilter: RandomFilter;
  bucketId: string;
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
  if (bucketId !== "all") {
    params.set("bucketId", bucketId);
  }

  return params.toString();
}

function isVideoUrl(url: string) {
  const base = url.split("?")[0].toLowerCase();
  return base.endsWith(".mp4") || base.endsWith(".webm");
}

function parseTagInput(value: string): string[] {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);
}

function clampRandomWeight(value: number): number {
  return Math.min(10, Math.max(0, Math.round(value)));
}

function formatAddedAt(value?: string) {
  if (!value) return "Added recently";
  try {
    const date = new Date(value);
    return `Added on ${date.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    })}`;
  } catch {
    return "Added recently";
  }
}
