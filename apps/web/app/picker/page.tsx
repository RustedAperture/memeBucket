"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Folder, Settings, X, Minus, Move } from "lucide-react";
import { apiGet } from "@/lib/api";
import type { ImageSearchResult, Bucket } from "@/lib/types";
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

export default function PickerPage() {
  const [query, setQuery] = useState("");
  const [bucketId, setBucketId] = useState("all");
  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [results, setResults] = useState<ImageSearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showSettings, setShowSettings] = useState(false);
  const [serverUrl, setServerUrl] = useState("");
  const [isTauriApp, setIsTauriApp] = useState(false);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const urlInputRef = useRef<HTMLInputElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    setServerUrl(window.location.origin);
    setIsTauriApp(isTauri());
  }, []);

  useEffect(() => {
    if (showSettings) urlInputRef.current?.focus();
    else searchInputRef.current?.focus();
  }, [showSettings]);

  const bucketItems = useMemo(
    () => [
      { label: "All buckets", value: "all" },
      ...buckets.map((b) => ({ label: b.name, value: b.id })),
    ],
    [buckets]
  );

  useEffect(() => {
    let cancelled = false;
    apiGet<Bucket[]>("/api/buckets")
      .then((loaded) => {
        if (!cancelled) setBuckets(loaded);
      })
      .catch(() => toast.error("Could not load buckets"));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timeout = setTimeout(() => {
      setLoading(true);
      const params = new URLSearchParams();
      params.set("limit", "40");
      if (query.trim()) params.set("q", query.trim());
      if (bucketId !== "all") params.set("bucketId", bucketId);

      apiGet<ImageSearchResult[]>(`/api/images/search?${params.toString()}`)
        .then((loaded) => {
          if (!cancelled) {
            setResults(loaded);
            setSelectedIndex(0);
          }
        })
        .catch(() => {
          if (!cancelled) setResults([]);
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 150);

    return () => {
      cancelled = true;
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

  const handleSaveUrl = async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const normalized = await invoke<string>("set_server_url", { url: serverUrl });
      // Fire-and-forget: navigation tears down the page before the response
      // arrives, so awaiting causes a spurious rejection.
      invoke("navigate_to", { url: `${normalized}/picker` }).catch(() => {});
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(`Failed to save: ${msg}`);
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (showSettings) {
        if (e.key === "Escape") {
          e.preventDefault();
          setShowSettings(false);
        }
        return;
      }

      if (results.length === 0) return;

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
        case "Escape":
          e.preventDefault();
          if (isTauri()) {
            import("@tauri-apps/api/core").then(({ invoke }) => {
              invoke("hide_window");
            });
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
  }, [selectedIndex, results, showSettings]);

  useEffect(() => {
    searchInputRef.current?.focus();
  }, []);

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
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Type to search your buckets..."
            className="h-8 pl-8 text-base md:text-sm rounded-md"
          />
        </div>

        <div className="flex items-center justify-between gap-2">
          <Select
            items={bucketItems}
            value={bucketId}
            onValueChange={(value) => {
              if (typeof value === "string") setBucketId(value);
            }}
          >
            <SelectTrigger className="h-7 text-xs gap-1.5 px-2 rounded-md min-w-0 flex-1">
              <Folder className="h-3 w-3 text-muted-foreground shrink-0" />
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="min-w-[200px]">
              <SelectGroup>
                <SelectItem value="all">All buckets</SelectItem>
                {buckets.map((bucket) => (
                  <SelectItem key={bucket.id} value={bucket.id}>
                    {bucket.name}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>

          <div className="flex items-center gap-1 shrink-0">
            <Button
              size="icon"
              onClick={() => setShowSettings((s) => !s)}
              className={`text-muted-foreground hover:text-foreground ${showSettings ? "text-foreground bg-muted" : ""}`}
              title="Change server URL"
            >
              {showSettings ? <X className="h-3 w-3" /> : <Settings className="h-3 w-3" />}
            </Button>
          </div>
        </div>

        {showSettings && (
          <div className="flex items-center gap-1.5">
            <Input
              ref={urlInputRef}
              type="text"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSaveUrl();
                if (e.key === "Escape") setShowSettings(false);
              }}
              placeholder="https://your-server.com"
              className="h-7 text-xs rounded-md"
            />
            <Button
              size="sm"
              className="h-7 px-2.5 text-xs rounded-md shrink-0"
              onClick={handleSaveUrl}
            >
              Save
            </Button>
          </div>
        )}
      </div>

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
                  {result.image.cdn_status === 'broken' ? (
                    <div className="flex items-center justify-center w-full h-full bg-muted rounded text-muted-foreground text-xs p-2 text-center" style={{ minHeight: '80px' }}>
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
    </div>
  );
}
