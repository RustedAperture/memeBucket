"use client";

import { Folder, Plus, Users, Globe } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet, apiPost } from "@/lib/api";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { PoolForm } from "@/components/pool-form";
import { cn } from "@/lib/utils";

import {
  SidebarHeader,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuBadge,
  useSidebar,
} from "@/components/ui/sidebar";

import { Pool } from "@/lib/types";
import { toast } from "sonner";

type DraggedImagesPayload = {
  imageId?: string;
  imageIds?: string[];
  sourcePoolId?: string;
};

export function PoolList({ onPoolsChange, onImageMoved, refreshKey }: { onPoolsChange?: (pools: Pool[]) => void, onImageMoved?: () => void, refreshKey?: number }) {
  const [pools, setPools] = useState<Pool[]>([]);
  const [error, setError] = useState<string | null>(null);
  const searchParams = useSearchParams();
  const activeId = searchParams.get("id");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  
  // useSidebar must be inside a component wrapped by SidebarProvider
  const { setOpenMobile } = useSidebar();

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
    try {
      const [loaded, favoritesResult] = await Promise.all([
        apiGet<Pool[]>("/api/pools"),
        apiGet<any[]>("/api/images/search?favoriteOnly=true&limit=1").catch(() => []),
      ]);

      let finalPools = [...loaded];

      // Inject virtual Favorites pool at the top if there are favorites
      if (favoritesResult.length > 0) {
        finalPools.unshift({
          id: "favorites",
          name: "Favorites",
          share_token: null,
          subscriber_count: 0,
          is_subscribed: false,
          owner_username: null,
          whitelist_enabled: false,
          image_count: favoritesResult.length, // we only fetch 1 to know it's not empty
          is_read_only: true,
        });
      }

      setPools(finalPools);
      if (onPoolsChange) onPoolsChange(finalPools);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not load pools");
    }
  }

  useEffect(() => {
    void load();
  }, [refreshKey, onPoolsChange]);



  return (
    <>
      <SidebarHeader className="flex flex-row h-14 items-center justify-between pl-4 pr-2 border-b">
        <span className="font-semibold text-lg">Pools</span>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger render={
            <Button variant="outline" size="icon" className="h-8 w-8 relative z-10" title="Add New Pool">
              <Plus className="w-4 h-4" />
              <span className="sr-only">Add New Pool</span>
            </Button>
          } />
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add New Pool</DialogTitle>
            </DialogHeader>
            <PoolForm onCreated={() => { setDialogOpen(false); void load(); }} />
          </DialogContent>
        </Dialog>
      </SidebarHeader>
      <SidebarContent className="bg-transparent pt-2">
        <SidebarGroup>
          <SidebarGroupContent>
        {error ? <p className="text-sm font-medium text-destructive px-2 pb-2">{error}</p> : null}
        {pools.length === 0 ? (
          <p className="text-sm text-muted-foreground p-4 text-center">No pools yet.</p>
        ) : (
          <SidebarMenu>
            {pools.map((pool) => {
              const isActive = pool.id === activeId;
              const hasBadge = !pool.is_subscribed && pool.share_token && pool.subscriber_count > 0;
              
              // We adjust padding right if it has a badge, but SidebarMenuAction handles its own overlap
              return (
                <SidebarMenuItem 
                  key={pool.id}
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
                  <SidebarMenuButton
                    render={<Link href={`/pools?id=${pool.id}`} onClick={() => setOpenMobile(false)} />}
                    isActive={isActive}
                    className={cn(
                      dragOverId === pool.id && "ring-2 ring-primary bg-sidebar-accent",
                      isActive && "!bg-primary !text-primary-foreground hover:!bg-primary/90",
                      hasBadge && "pr-12" // leave room for badge
                    )}
                  >
                    {pool.is_subscribed ? <Globe /> : <Folder />}
                    <span>{pool.name}</span>
                  </SidebarMenuButton>

                  {hasBadge && (
                    <SidebarMenuBadge className={cn("gap-1 bg-muted/50 font-normal px-1.5 right-2", isActive && "text-primary-foreground bg-primary-foreground/20")}>
                      <Users className="w-3 h-3" />
                      <span>{pool.subscriber_count}</span>
                    </SidebarMenuBadge>
                  )}
                </SidebarMenuItem>
              );
            })}
          </SidebarMenu>
        )}
      </SidebarGroupContent>
    </SidebarGroup>
    </SidebarContent>
    </>
  );
}
