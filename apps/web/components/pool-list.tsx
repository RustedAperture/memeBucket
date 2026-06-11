"use client";

import { Folder, Plus, Trash2, X, Users, Globe } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet, apiPost } from "@/lib/api";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { PoolForm } from "@/components/pool-form";
import { SheetClose } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";

import { Pool } from "@/lib/types";
import { toast } from "sonner";

type DraggedImagesPayload = {
  imageId?: string;
  imageIds?: string[];
  sourcePoolId?: string;
};

export function PoolList({ isMobile, onPoolsChange, onImageMoved }: { isMobile?: boolean, onPoolsChange?: (pools: Pool[]) => void, onImageMoved?: () => void }) {
  const [pools, setPools] = useState<Pool[]>([]);
  const [error, setError] = useState<string | null>(null);
  const searchParams = useSearchParams();
  const activeId = searchParams.get("id");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dragOverId, setDragOverId] = useState<string | null>(null);

  async function handleDrop(e: React.DragEvent, targetPool: Pool) {
    e.preventDefault();
    setDragOverId(null);
    if (targetPool.is_subscribed) return;

    try {
      const dataStr = e.dataTransfer.getData("application/json");
      if (!dataStr) return;
      
      const data = JSON.parse(dataStr) as DraggedImagesPayload;
      const imageIds = data.imageIds?.length ? data.imageIds : data.imageId ? [data.imageId] : [];
      if (data.sourcePoolId === targetPool.id) return;
      if (!data.sourcePoolId || imageIds.length === 0) return;
      
      const results = await Promise.allSettled(
        imageIds.map((imageId) =>
          apiPost(`/api/pools/${data.sourcePoolId}/images/${imageId}/move`, { new_pool_id: targetPool.id })
        )
      );
      const movedCount = results.filter((result) => result.status === "fulfilled").length;
      if (onImageMoved) onImageMoved();
      if (movedCount === imageIds.length) {
        toast.success(`${movedCount} image${movedCount === 1 ? "" : "s"} moved successfully`);
      } else if (movedCount > 0) {
        toast.warning(`Moved ${movedCount} of ${imageIds.length} images`);
      } else {
        toast.error(`Failed to move ${imageIds.length === 1 ? "image" : "images"}`);
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to move images");
    }
  }

  async function load() {
    const loaded = await apiGet<Pool[]>("/api/pools");
    setPools(loaded);
    if (onPoolsChange) onPoolsChange(loaded);
  }

  useEffect(() => {
    let cancelled = false;
    void apiGet<Pool[]>("/api/pools")
      .then((loaded) => {
        if (cancelled) return;
        setPools(loaded);
        if (onPoolsChange) onPoolsChange(loaded);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Could not load pools");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [onPoolsChange]);

  async function handleDelete(pool: Pool) {
    setError(null);
    try {
      if (pool.is_subscribed) {
        await apiPost(`/api/pools/${pool.id}/unsubscribe`, {});
      } else {
        await apiDelete(`/api/pools/${pool.id}`);
      }
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not remove pool");
    }
  }

  return (
    <div className="flex flex-col h-full bg-sidebar/50 text-sidebar-foreground">
      <div className="flex h-14 items-center justify-between pl-4 pr-2">
        <span className="font-semibold text-lg">Pools</span>
        
        <div className={cn("flex items-center", isMobile && "-space-x-px")}>
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger render={<Button variant="outline" size="icon" className={cn("h-8 w-8 relative z-10", isMobile && "rounded-r-none hover:z-20")} />}>
              <Plus className="w-4 h-4" />
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Add New Pool</DialogTitle>
              </DialogHeader>
              <PoolForm onCreated={() => { setDialogOpen(false); void load(); }} />
            </DialogContent>
          </Dialog>
          {isMobile && (
            <SheetClose render={<Button variant="outline" size="icon" className="h-8 w-8 rounded-l-none relative z-10 hover:z-20" />}>
              <X className="w-4 h-4" />
            </SheetClose>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-4 pt-2">
        <div className="mb-4 h-px bg-border" />
        {error ? <p className="text-sm font-medium text-destructive px-2 pb-2">{error}</p> : null}
        {pools.length === 0 ? (
          <p className="text-sm text-muted-foreground p-4 text-center">No pools yet.</p>
        ) : (
          <div className="space-y-1">
            {pools.map((pool) => {
              const isActive = pool.id === activeId;
              return (
                <div 
                  key={pool.id} 
                  className="group relative"
                  onDragOver={(e) => {
                    if (!pool.is_subscribed) {
                      e.preventDefault();
                      setDragOverId(pool.id);
                    }
                  }}
                  onDragLeave={() => {
                    setDragOverId(null);
                  }}
                  onDrop={(e) => handleDrop(e, pool)}
                >
                  <Link 
                    href={`/pools?id=${pool.id}`} 
                    className={`flex h-8 items-center gap-2 overflow-hidden rounded-md px-2 text-sm ring-sidebar-ring outline-hidden transition-all ${
                      isActive 
                        ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground" 
                        : "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                    } ${dragOverId === pool.id ? "ring-2 ring-primary" : ""}`}
                  >
                    {pool.is_subscribed ? (
                      <Globe className="h-4 w-4 shrink-0" />
                    ) : (
                      <Folder className="h-4 w-4 shrink-0" />
                    )}
                    <span className="truncate flex-1">{pool.name}</span>
                    {!pool.is_subscribed && pool.share_token && pool.subscriber_count > 0 && (
                      <div className="flex items-center gap-1 text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded-sm mr-6">
                        <Users className="w-3 h-3" />
                        <span>{pool.subscriber_count}</span>
                      </div>
                    )}
                  </Link>
                  <Button 
                    variant="ghost" 
                    size="icon" 
                    title={pool.is_subscribed ? "Unsubscribe" : "Delete"}
                    className={`absolute right-1 top-1 h-6 w-6 rounded-md p-0 opacity-0 transition-opacity hover:bg-muted focus-visible:opacity-100 group-hover:opacity-100 ${
                      isActive ? "text-sidebar-accent-foreground" : "text-muted-foreground"
                    }`}
                    onClick={(e) => { e.preventDefault(); handleDelete(pool); }}
                  >
                    {pool.is_subscribed ? (
                      <X className="w-3 h-3 text-muted-foreground" />
                    ) : (
                      <Trash2 className="w-3 h-3 text-destructive" />
                    )}
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
