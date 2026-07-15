"use client";

import { Folder, Plus, Users, Globe, Star, Images, Inbox } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet, apiPost } from "@/lib/api";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { BucketForm } from "@/components/bucket-form";
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

import { Bucket } from "@/lib/types";
import { toast } from "sonner";

type DraggedImagesPayload = {
  imageId?: string;
  imageIds?: string[];
  sourceBucketId?: string;
};

export function BucketList({ onBucketsChange, onImageMoved, refreshKey }: { onBucketsChange?: (buckets: Bucket[]) => void, onImageMoved?: () => void, refreshKey?: number }) {
  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [error, setError] = useState<string | null>(null);
  const searchParams = useSearchParams();
  const activeId = searchParams.get("id");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  
  // useSidebar must be inside a component wrapped by SidebarProvider
  const { setOpenMobile } = useSidebar();

  async function handleDrop(e: React.DragEvent, targetBucket: Bucket) {
    e.preventDefault();
    setDragOverId(null);
    if (targetBucket.is_subscribed || targetBucket.is_read_only) return;

    try {
      const dataStr = e.dataTransfer.getData("application/json");
      if (!dataStr) return;
      
      const data = JSON.parse(dataStr) as DraggedImagesPayload;
      const imageIds = data.imageIds?.length ? data.imageIds : data.imageId ? [data.imageId] : [];
      if (data.sourceBucketId === targetBucket.id) return;
      if (!data.sourceBucketId || imageIds.length === 0) return;
      
      const results = await Promise.allSettled(
        imageIds.map((imageId) =>
          apiPost(`/api/buckets/${data.sourceBucketId}/images/${imageId}/move`, { new_bucket_id: targetBucket.id })
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
        apiGet<Bucket[]>("/api/buckets"),
        apiGet<any[]>("/api/images/search?favorite=true&limit=1").catch(() => []),
      ]);

      const finalBuckets: Bucket[] = [{
        id: "all",
        name: "All",
        share_token: null,
        subscriber_count: 0,
        is_subscribed: false,
        owner_username: null,
        whitelist_enabled: false,
        image_count: loaded.reduce((total, bucket) => total + bucket.image_count, 0),
        is_read_only: true,
      }, ...loaded];

      // Inject virtual Favorites bucket at the top if there are favorites
      if (favoritesResult.length > 0) {
        finalBuckets.unshift({
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

      finalBuckets.sort((left, right) => {
        const rank = (bucket: Bucket) => {
          if (bucket.id === "all") return 0;
          if (bucket.id === "favorites") return 1;
          if (bucket.name.toLowerCase() === "inbox") return 2;
          return 3;
        };
        return rank(left) - rank(right) || left.name.localeCompare(right.name);
      });

      setBuckets(finalBuckets);
      if (onBucketsChange) onBucketsChange(finalBuckets);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not load buckets");
    }
  }

  useEffect(() => {
    void load();
  }, [refreshKey, onBucketsChange]);



  return (
    <>
      <SidebarHeader className="flex flex-row h-14 items-center justify-between pl-4 pr-2 border-b">
        <span className="font-semibold text-lg">Buckets</span>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger render={
            <Button variant="outline" size="icon" className="h-8 w-8 relative z-10" title="Add New Bucket">
              <Plus className="w-4 h-4" />
              <span className="sr-only">Add New Bucket</span>
            </Button>
          } />
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add New Bucket</DialogTitle>
            </DialogHeader>
            <BucketForm onCreated={(bucket) => {
              setDialogOpen(false);
              if (bucket) {
                setBuckets((current) => {
                  const next = [...current, bucket];
                  next.sort((left, right) => left.name.localeCompare(right.name));
                  return next;
                });
              }
              void load();
            }} />
          </DialogContent>
        </Dialog>
      </SidebarHeader>
      <SidebarContent className="bg-transparent pt-2">
        <SidebarGroup>
          <SidebarGroupContent>
        {error ? <p className="text-sm font-medium text-destructive px-2 pb-2">{error}</p> : null}
        {buckets.length === 0 ? (
          <p className="text-sm text-muted-foreground p-4 text-center">No buckets yet.</p>
        ) : (
          <SidebarMenu>
            {buckets.map((bucket) => {
              const isActive = bucket.id === activeId;
              const hasBadge = !bucket.is_subscribed && bucket.share_token && bucket.subscriber_count > 0;
              
              // We adjust padding right if it has a badge, but SidebarMenuAction handles its own overlap
              return (
                <SidebarMenuItem 
                  key={bucket.id}
                  onDragOver={(e) => {
                    if (!bucket.is_subscribed) {
                      e.preventDefault();
                      setDragOverId(bucket.id);
                    }
                  }}
                  onDragLeave={() => {
                    setDragOverId(null);
                  }}
                  onDrop={(e) => handleDrop(e, bucket)}
                >
                  <SidebarMenuButton
                    render={<Link href={`/buckets?id=${bucket.id}`} onClick={() => setOpenMobile(false)} />}
                    isActive={isActive}
                    className={cn(
                      dragOverId === bucket.id && "ring-2 ring-primary bg-sidebar-accent",
                      isActive && "!bg-primary !text-primary-foreground hover:!bg-primary/90",
                      hasBadge && "pr-12" // leave room for badge
                    )}
                  >
                    {bucket.id === "all" ? <Images /> : bucket.id === "favorites" ? <Star /> : bucket.name.toLowerCase() === "inbox" ? <Inbox /> : bucket.is_subscribed ? <Globe /> : <Folder />}
                    <span>{bucket.name}</span>
                  </SidebarMenuButton>

                  {hasBadge && (
                    <SidebarMenuBadge className={cn("gap-1 bg-muted/50 font-normal px-1.5 right-2", isActive && "text-primary-foreground bg-primary-foreground/20")}>
                      <Users className="w-3 h-3" />
                      <span>{bucket.subscriber_count}</span>
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
