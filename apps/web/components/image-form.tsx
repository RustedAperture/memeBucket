"use client";

import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { apiPost } from "@/lib/api";
import type { GifSearchSelection } from "@/lib/types";
import { Plus, Search, X, ListPlus, Loader2, AlertCircle, CheckCircle2 } from "lucide-react";
import { GifSearchModal } from "./gif-search-modal";
import { ButtonGroup } from "@/components/ui/button-group";
import { toast } from "sonner";

type SubmissionKind = "image" | "video" | "twitter" | "bluesky";

// Client-side heuristic for the loading copy. The server remains authoritative
// about resolution and conversion; this only identifies the submitted URL's
// broad category while the request is in flight.
function classifySubmission(url: string): SubmissionKind {
  try {
    const parsed = new URL(url);
    const path = parsed.pathname.toLowerCase();
    const host = parsed.hostname.toLowerCase().replace(/^www\./, "");
    if (host === "x.com" || host === "twitter.com" || host === "mobile.twitter.com") {
      return "twitter";
    }
    if (host === "bsky.app" || host === "bsky.social") return "bluesky";
    if (path.endsWith(".mp4") || path.endsWith(".webm") || path.endsWith(".m3u8")) {
      return "video";
    }
    return "image";
  } catch {
    return "image";
  }
}

export function ImageForm({ bucketId, onCreated }: { bucketId: string; onCreated: () => void }) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [creatingKind, setCreatingKind] = useState<SubmissionKind>("image");
  const creatingRef = useRef(false);

  // Bulk Add States
  const [bulkOpen, setBulkOpen] = useState(false);
  const [bulkUrls, setBulkUrls] = useState("");
  const [isBulkAdding, setIsBulkAdding] = useState(false);
  const [bulkProgress, setBulkProgress] = useState<{ current: number; total: number } | null>(null);
  const [bulkCurrentUrl, setBulkCurrentUrl] = useState<string | null>(null);
  const [bulkErrors, setBulkErrors] = useState<{ url: string; error: string }[]>([]);
  const [bulkSuccessCount, setBulkSuccessCount] = useState(0);

  async function createImage(payload: { url: string; title?: string; tags?: string[] }) {
    if (creatingRef.current) {
      return false;
    }
    creatingRef.current = true;
    setIsCreating(true);
    setCreatingKind(classifySubmission(payload.url));
    try {
      await apiPost(`/api/buckets/${bucketId}/images`, payload);
      setUrl("");
      onCreated();
      return true;
    } catch (err) {
      throw err;
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

  async function handleBulkSubmit() {
    const urls = bulkUrls
      .split(/[\n,]/)
      .map((line) => line.trim())
      .filter((line) => {
        if (!line) return false;
        try {
          new URL(line);
          return true;
        } catch {
          return false;
        }
      });

    if (urls.length === 0) {
      toast.error("Please enter at least one valid URL.");
      return;
    }

    setIsBulkAdding(true);
    setBulkErrors([]);
    setBulkSuccessCount(0);
    setBulkProgress({ current: 0, total: urls.length });

    const failed: { url: string; error: string }[] = [];
    let successCount = 0;

    for (let i = 0; i < urls.length; i++) {
      const currentUrl = urls[i];
      setBulkProgress({ current: i + 1, total: urls.length });
      setBulkCurrentUrl(currentUrl);
      try {
        await apiPost(`/api/buckets/${bucketId}/images`, { url: currentUrl });
        successCount++;
        setBulkSuccessCount(successCount);
      } catch (err) {
        failed.push({
          url: currentUrl,
          error: err instanceof Error ? err.message : "Failed to add image",
        });
        setBulkErrors([...failed]);
      }
    }

    setIsBulkAdding(false);
    setBulkProgress(null);
    setBulkCurrentUrl(null);
    onCreated();

    if (failed.length === 0) {
      toast.success(`Successfully added all ${successCount} images.`);
      setBulkUrls("");
      setBulkOpen(false);
    } else {
      toast.warning(`Added ${successCount} images, but ${failed.length} failed.`);
      // Update text area with only the failed URLs so they can correct/retry them easily
      setBulkUrls(failed.map((f) => f.url).join("\n"));
    }
  }

  return (
    <>
      <div className="relative w-full">
        <form onSubmit={submit} className="flex items-center gap-2 w-full">
          <Input
            value={url}
            onChange={(event) => handleUrlChange(event.target.value)}
            placeholder="URL or search KLIPY..."
            className="h-8 flex-1 sm:w-48 text-sm"
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
            <Button
              type="button"
              variant="default"
              size="icon"
              onClick={() => {
                setBulkErrors([]);
                setBulkSuccessCount(0);
                setBulkUrls("");
                setBulkOpen(true);
              }}
              title="Bulk Add Links"
              disabled={isCreating}
            >
              <ListPlus className="w-4 h-4" />
            </Button>
          </ButtonGroup>
        </form>
        {error ? (
          <p className="absolute top-full mt-1 right-0 z-40 text-xs font-medium text-destructive whitespace-nowrap">
            {error}
          </p>
        ) : null}
      </div>

      {/* Bulk Add Dialog */}
      <Dialog
        open={bulkOpen}
        onOpenChange={(open) => {
          if (!isBulkAdding) {
            setBulkOpen(open);
          }
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Bulk Add Links</DialogTitle>
            <DialogDescription>
              Paste image or video URLs below (one per line, or separated by commas).
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 my-2">
            {!isBulkAdding && (
              <Textarea
                placeholder="https://example.com/image1.gif&#10;https://example.com/image2.mp4"
                value={bulkUrls}
                onChange={(e) => setBulkUrls(e.target.value)}
                className="h-48 resize-none font-mono text-xs"
              />
            )}

            {isBulkAdding && bulkProgress && (
              <div className="flex flex-col items-center justify-center p-8 space-y-4 border rounded-lg bg-muted/20">
                <Loader2 className="w-8 h-8 animate-spin text-primary" />
                <div className="text-center w-full min-w-0">
                  <p className="text-sm font-medium">
                    Adding links... ({bulkProgress.current} / {bulkProgress.total})
                  </p>
                  {bulkCurrentUrl ? (
                    <p className="text-xs text-muted-foreground mt-1 truncate" title={bulkCurrentUrl}>
                      {bulkCurrentUrl}
                    </p>
                  ) : null}
                  <p className="text-xs text-muted-foreground mt-1">
                    Successfully added: {bulkSuccessCount}
                  </p>
                  <p className="text-[11px] text-muted-foreground/80 mt-2">
                    Videos and X/Twitter links can take longer per item to convert — this is still working even if the counter pauses.
                  </p>
                </div>
                <div className="w-full bg-border rounded-full h-1.5 overflow-hidden">
                  <div
                    className="bg-primary h-1.5 transition-all duration-150"
                    style={{ width: `${(bulkProgress.current / bulkProgress.total) * 100}%` }}
                  />
                </div>
              </div>
            )}

            {bulkErrors.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-xs font-semibold text-destructive flex items-center gap-1.5">
                  <AlertCircle className="w-3.5 h-3.5" />
                  Failed Imports ({bulkErrors.length})
                </h4>
                <div className="max-h-32 overflow-y-auto border rounded-md p-2 bg-destructive/5 space-y-1.5">
                  {bulkErrors.map((item, idx) => (
                    <div key={idx} className="text-xs font-mono border-b border-destructive/10 pb-1 last:border-0 last:pb-0">
                      <div className="text-muted-foreground truncate" title={item.url}>{item.url}</div>
                      <div className="text-destructive font-medium">{item.error}</div>
                    </div>
                  ))}
                </div>
                {!isBulkAdding && (
                  <p className="text-xs text-muted-foreground">
                    The text area has been updated with only the failed URLs. Fix any issues and click Import again.
                  </p>
                )}
              </div>
            )}

            {bulkSuccessCount > 0 && bulkErrors.length === 0 && !isBulkAdding && (
              <div className="flex items-center gap-2 p-3 border rounded-lg bg-green-500/10 border-green-500/20 text-green-600 text-sm">
                <CheckCircle2 className="w-4 h-4 shrink-0 text-green-500" />
                Successfully imported {bulkSuccessCount} images!
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setBulkOpen(false)}
              disabled={isBulkAdding}
            >
              Cancel
            </Button>
            <Button
              onClick={handleBulkSubmit}
              disabled={isBulkAdding || !bulkUrls.trim()}
            >
              {isBulkAdding ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Adding...
                </>
              ) : (
                "Import Links"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Adding-image progress modal (single URL / GIF search select) */}
      <Dialog open={isCreating} onOpenChange={() => {}}>
        <DialogContent className="sm:max-w-sm" showCloseButton={false}>
          <div className="flex flex-col items-center justify-center gap-3 py-4 text-center">
            <Loader2 className="w-8 h-8 animate-spin text-primary" />
            <div>
              <p className="text-sm font-medium">
                {creatingKind === "video"
                  ? "Adding video…"
                  : creatingKind === "twitter"
                    ? "Adding X/Twitter post…"
                    : creatingKind === "bluesky"
                      ? "Adding Bluesky post…"
                      : "Adding image…"}
              </p>
              {creatingKind !== "image" ? (
                <p className="text-xs text-muted-foreground mt-1">
                  {creatingKind === "video"
                    ? "Videos can take up to a minute to convert."
                    : creatingKind === "twitter"
                      ? "X/Twitter links can take up to a minute to resolve and convert."
                      : "Bluesky links can take up to a minute to resolve and convert."}
                </p>
              ) : null}
            </div>
          </div>
        </DialogContent>
      </Dialog>

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
