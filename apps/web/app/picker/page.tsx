"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Folder, X, Minus, Move, Plus } from "lucide-react";
import { PickerAddLinks } from "@/components/picker-add-links";
import { apiGet } from "@/lib/api";
import type { ImageSearchResult, Bucket } from "@/lib/types";
import { isWritablePickerBucket } from "@/lib/picker-add-links";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const isTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const PICKER_BUCKET_STORAGE_KEY = "picker.selectedBucketId";
const PICKER_ALL_BUCKET_ID = "all";
const PICKER_INBOX_NAME = "inbox";

export default function PickerPage() {
  const [query, setQuery] = useState("");
  const [bucketId, setBucketId] = useState(PICKER_ALL_BUCKET_ID);
  const [pickerMode, setPickerMode] = useState<"search" | "add-links">("search");
  const [isAddLinksSubmitting, setIsAddLinksSubmitting] = useState(false);
  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [results, setResults] = useState<ImageSearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isTauriApp, setIsTauriApp] = useState(false);
  const [changelogBanner, setChangelogBanner] = useState<{ version: string; date: string } | null>(null);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);
  const queryRef = useRef(query);
  const bucketIdRef = useRef(bucketId);
  const searchRequestId = useRef(0);
  const storedBucketIdRef = useRef<string | null>(null);
  const hasRestoredStoredBucketRef = useRef(false);

  useEffect(() => {
    queryRef.current = query;
    bucketIdRef.current = bucketId;
  }, [query, bucketId]);

  useEffect(() => {
    setIsTauriApp(isTauri());
  }, []);

  useEffect(() => {
    storedBucketIdRef.current = window.localStorage.getItem(PICKER_BUCKET_STORAGE_KEY);
  }, []);

  useEffect(() => {
    async function checkChangelog() {
      try {
        const response = await fetch("/changelog.json", { cache: "no-store" });
        if (!response.ok) return;
        const data: { version: string; date: string }[] = await response.json();
        const latest = data[0];
        if (!latest) return;

        const lastSeen = localStorage.getItem("lastSeenChangelogVersion");
        if (lastSeen !== latest.version) {
          localStorage.setItem("lastSeenChangelogVersion", latest.version);
          setChangelogBanner({ version: latest.version, date: latest.date });
        }
      } catch {
        // Offline or transient failure — the banner just doesn't show this time.
      }
    }

    checkChangelog();
    window.addEventListener("focus", checkChangelog);
    return () => window.removeEventListener("focus", checkChangelog);
  }, []);

  const bucketItems = useMemo(
    () => [
      { label: "All buckets", value: PICKER_ALL_BUCKET_ID },
      ...buckets.map((b) => ({ label: b.name, value: b.id })),
    ],
    [buckets]
  );

  const ownedInboxBucket = useMemo(
    () =>
      buckets.find(
        (bucket) =>
          bucket.name.trim().toLowerCase() === PICKER_INBOX_NAME &&
          !bucket.is_subscribed
      ) ?? null,
    [buckets]
  );

  function setBucketSelection(nextBucketId: string, nextBuckets: Bucket[] = buckets) {
    const nextValue =
      nextBucketId === PICKER_ALL_BUCKET_ID || nextBuckets.some((bucket) => bucket.id === nextBucketId)
        ? nextBucketId
        : PICKER_ALL_BUCKET_ID;

    setBucketId(nextValue);

    if (typeof window === "undefined") return;

    if (isWritablePickerBucket(nextValue, nextBuckets)) {
      window.localStorage.setItem(PICKER_BUCKET_STORAGE_KEY, nextValue);
      storedBucketIdRef.current = nextValue;
    }
  }

  async function fetchBuckets() {
    try {
      const loaded = await apiGet<Bucket[]>("/api/buckets");
      setBuckets(loaded);
      if (!hasRestoredStoredBucketRef.current) {
        hasRestoredStoredBucketRef.current = true;
        const storedBucketId = storedBucketIdRef.current;
        if (storedBucketId && isWritablePickerBucket(storedBucketId, loaded)) {
          setBucketId(storedBucketId);
        }
      }
    } catch {
      toast.error("Could not load buckets");
    }
  }

  // Guarded by a monotonic request id (not a per-effect `cancelled` flag) since
  // this is called from two independent triggers — debounced typing and window
  // focus — and only the response to the most recently issued request should
  // ever be allowed to update state.
  async function fetchResults() {
    const requestId = ++searchRequestId.current;
    setLoading(true);
    const params = new URLSearchParams();
    params.set("limit", "40");
    if (queryRef.current.trim()) params.set("q", queryRef.current.trim());
    if (bucketIdRef.current !== "all") params.set("bucketId", bucketIdRef.current);

    try {
      const loaded = await apiGet<ImageSearchResult[]>(`/api/images/search?${params.toString()}`);
      if (requestId === searchRequestId.current) {
        setResults(loaded);
        setSelectedIndex(0);
      }
    } catch {
      if (requestId === searchRequestId.current) setResults([]);
    } finally {
      if (requestId === searchRequestId.current) setLoading(false);
    }
  }

  useEffect(() => {
    fetchBuckets();
  }, []);

  // The desktop Picker's hotkey only toggles native window visibility
  // (hide()/show()) — it never reloads the page, so without this, data
  // fetched once at mount would go stale for the lifetime of the app
  // session if images/buckets changed from another surface while hidden.
  useEffect(() => {
    function refreshOnFocus() {
      fetchBuckets();
      fetchResults();
    }
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, []);

  useEffect(() => {
    const timeout = setTimeout(() => {
      fetchResults();
    }, 150);

    return () => {
      clearTimeout(timeout);
    };
  }, [query, bucketId]);

  const handleMinimize = async () => {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    await getCurrentWebviewWindow().minimize();
  };

  const handleClose = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("hide_window");
  };

  const handleOpenChangelog = async () => {
    const changelogUrl = `${window.location.origin}/changelog`;
    if (isTauri()) {
      try {
        const { open } = await import("@tauri-apps/plugin-shell");
        await open(changelogUrl);
      } catch {
        window.open(changelogUrl, "_blank");
      }
    } else {
      window.open(changelogUrl, "_blank");
    }
    setChangelogBanner(null);
  };

  const handleSelectImage = async (url: string) => {
    if (isTauri()) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("copy_and_paste_link", { url });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(`Paste failed: ${msg}`);
      }
    } else {
      try {
        await navigator.clipboard.writeText(url);
        toast.success("Copied link to clipboard");
      } catch {
        toast.error("Failed to copy link");
      }
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (isTauri()) {
          import("@tauri-apps/api/core").then(({ invoke }) => {
            invoke("hide_window");
          });
        }
        return;
      }

      if (pickerMode !== "search" || results.length === 0) return;

      let nextIndex = selectedIndex;

      const inputFocused = document.activeElement === searchInputRef.current;

      switch (e.key) {
        case "ArrowUp":
          e.preventDefault();
          nextIndex = Math.max(0, selectedIndex - 1);
          break;
        case "ArrowDown":
          e.preventDefault();
          nextIndex = Math.min(results.length - 1, selectedIndex + 1);
          break;
        case "ArrowLeft":
        case "ArrowRight": {
          // Let the search input handle cursor movement normally.
          if (inputFocused) return;
          e.preventDefault();
          const goRight = e.key === "ArrowRight";
          const currentEl = itemRefs.current[selectedIndex];
          if (!currentEl) break;
          const currentRect = currentEl.getBoundingClientRect();
          let bestDist = Infinity;
          itemRefs.current.forEach((el, idx) => {
            if (!el || idx === selectedIndex) return;
            const rect = el.getBoundingClientRect();
            const isRightOf = rect.left > currentRect.left + 1;
            if (goRight !== isRightOf) return;
            const dist = Math.abs(rect.top + rect.height / 2 - (currentRect.top + currentRect.height / 2));
            if (dist < bestDist) { bestDist = dist; nextIndex = idx; }
          });
          break;
        }
        case "Enter":
          e.preventDefault();
          if (results[selectedIndex]) {
            handleSelectImage(results[selectedIndex].image.url);
          }
          return;
        default:
          if (
            document.activeElement !== searchInputRef.current &&
            e.key.length === 1 &&
            !e.ctrlKey &&
            !e.metaKey &&
            !e.altKey
          ) {
            searchInputRef.current?.focus();
          }
          return;
      }

      if (nextIndex !== selectedIndex) {
        setSelectedIndex(nextIndex);
        itemRefs.current[nextIndex]?.scrollIntoView({
          block: "nearest",
          behavior: "smooth",
        });
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [pickerMode, selectedIndex, results]);

  useEffect(() => {
    if (pickerMode === "search") {
      searchInputRef.current?.focus();
    }
  }, [pickerMode]);

  useEffect(() => {
    if (!isTauriApp) return;
    let unlisten: (() => void) | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;

    (async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const { invoke } = await import("@tauri-apps/api/core");
      const win = getCurrentWebviewWindow();
      unlisten = await win.onMoved(() => {
        clearTimeout(timer);
        timer = setTimeout(async () => {
          const pos = await win.outerPosition();
          const scale = await win.scaleFactor();
          invoke("save_window_position", { x: pos.x / scale, y: pos.y / scale }).catch(() => {});
        }, 500);
      });
    })();

    return () => {
      unlisten?.();
      clearTimeout(timer);
    };
  }, [isTauriApp]);

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-foreground overflow-hidden select-none rounded-[12px]">
      {/* Drag handle (desktop only) */}
      {isTauriApp && (
        <div className="h-7 w-full shrink-0 flex items-center px-2 gap-2">
          <div
            data-tauri-drag-region
            className="flex-1 flex items-center cursor-grab active:cursor-grabbing"
          >
            <Move className="h-3.5 w-3.5 text-muted-foreground/50 pointer-events-none" />
          </div>
          <div className="flex items-center gap-0.5 shrink-0">
            <button
              onClick={handleMinimize}
              className="h-5 w-5 flex items-center justify-center rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
            >
              <Minus className="h-3 w-3" />
            </button>
            <button
              onClick={handleClose}
              className="h-5 w-5 flex items-center justify-center rounded hover:bg-destructive hover:text-destructive-foreground text-muted-foreground transition-colors"
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        </div>
      )}
      {/* Header */}
      <div className="px-2.5 pt-2.5 pb-2 border-b bg-card/40 backdrop-blur-md flex flex-col gap-2 shrink-0">
        <div className="relative flex items-center">
          <Search className="absolute left-2.5 h-3.5 w-3.5 text-muted-foreground pointer-events-none" />
          <Input
            ref={searchInputRef}
            type="text"
            value={query}
            disabled={pickerMode === "add-links" && isAddLinksSubmitting}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Type to search your buckets..."
            className="h-8 pl-8 text-base md:text-sm rounded-md"
          />
        </div>

        <div className="flex items-center justify-between gap-2">
          <div className="flex min-w-0 flex-1 items-center gap-2">
            <Select
              items={bucketItems}
              value={bucketId}
              disabled={pickerMode === "add-links" && isAddLinksSubmitting}
              onValueChange={(value) => {
                if (typeof value === "string") setBucketSelection(value);
              }}
            >
              <SelectTrigger className="h-7 text-xs gap-1.5 px-2 rounded-md min-w-0 flex-1">
                <Folder className="h-3 w-3 text-muted-foreground shrink-0" />
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="min-w-[200px]">
                <SelectGroup>
                  <SelectItem value={PICKER_ALL_BUCKET_ID}>All buckets</SelectItem>
                  {buckets.map((bucket) => (
                    <SelectItem key={bucket.id} value={bucket.id}>
                      {bucket.name}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>

            <Button
              type="button"
              variant="outline"
              size="icon"
              className="h-7 w-7 shrink-0 rounded-md"
              aria-label="Add media"
              title="Add media"
              disabled={isAddLinksSubmitting}
              onClick={() => setPickerMode("add-links")}
            >
              <Plus className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>

      {pickerMode === "add-links" ? (
        <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden bg-muted/10">
          <PickerAddLinks
            buckets={buckets}
            bucketId={bucketId}
            onBucketChange={(nextBucketId) => setBucketSelection(nextBucketId)}
            onUseInbox={() => {
              if (ownedInboxBucket) {
                setBucketSelection(ownedInboxBucket.id);
              }
            }}
            onBack={() => {
              if (!isAddLinksSubmitting) setPickerMode("search");
            }}
            onSubmissionStateChange={setIsAddLinksSubmitting}
          />
        </div>
      ) : (
        <>
          {changelogBanner && (
            <div className="flex items-center justify-between gap-2 px-2.5 py-1.5 bg-primary/10 border-b text-xs shrink-0">
              <button
                onClick={handleOpenChangelog}
                className="text-left flex-1 min-w-0 truncate hover:underline"
                aria-label={`View changelog for version ${changelogBanner.version}`}
              >
                New update available (v{changelogBanner.version}, {changelogBanner.date}) — see what&apos;s new
              </button>
              <button
                onClick={() => setChangelogBanner(null)}
                className="shrink-0 text-muted-foreground hover:text-foreground"
                aria-label="Dismiss"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          )}

          {/* Masonry image grid */}
          <div className="flex-grow min-h-0 overflow-y-auto p-2 scrollbar-none bg-muted/10">
            {loading && results.length === 0 ? (
              <div className="columns-2 gap-2">
                {Array.from({ length: 8 }).map((_, i) => (
                  <Skeleton
                    key={i}
                    className="break-inside-avoid mb-2 rounded-md"
                    style={{ height: `${100 + (i % 3) * 40}px` }}
                  />
                ))}
              </div>
            ) : results.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-center p-4 text-muted-foreground">
                <Search className="h-8 w-8 mb-2 opacity-40" />
                <p className="text-xs font-medium">No results found</p>
              </div>
            ) : (
              <div className="columns-2 gap-2 pb-2">
                {results.map((result, index) => {
                  const isSelected = index === selectedIndex;
                  const isVideo = result.image.url
                    .split("?")[0]
                    .toLowerCase()
                    .match(/\.(mp4|webm)$/);

                  return (
                    <div
                      key={result.image.id}
                      ref={(el) => {
                        itemRefs.current[index] = el;
                      }}
                      onClick={() => {
                        setSelectedIndex(index);
                        handleSelectImage(result.image.url);
                      }}
                      className={`break-inside-avoid mb-2 relative rounded-md overflow-hidden bg-muted cursor-pointer transition-all border-2 ${
                        isSelected
                          ? "border-primary ring-2 ring-primary ring-offset-1 ring-offset-background scale-[0.98]"
                          : "border-transparent hover:scale-[0.99] hover:border-muted-foreground/30"
                      }`}
                    >
                      {result.image.cdn_status === "broken" ? (
                        <div
                          className="flex items-center justify-center w-full h-full bg-muted rounded text-muted-foreground text-xs p-2 text-center"
                          style={{ minHeight: "80px" }}
                        >
                          <span>⚠ Link unavailable</span>
                        </div>
                      ) : isVideo ? (
                        <video
                          src={result.image.url}
                          autoPlay
                          loop
                          muted
                          playsInline
                          className="w-full h-auto pointer-events-none"
                        />
                      ) : (
                        <img
                          src={result.image.url}
                          alt={result.image.title || ""}
                          loading="lazy"
                          className="w-full h-auto pointer-events-none"
                        />
                      )}
                      {result.image.title && (
                        <div className="absolute bottom-0 left-0 right-0 p-1 bg-gradient-to-t from-black/80 to-transparent text-[10px] text-white truncate font-medium">
                          {result.image.title}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
