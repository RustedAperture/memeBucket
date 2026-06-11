"use client";

import { useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { AppShell } from "@/components/app-shell";
import { PoolList } from "@/components/pool-list";
import { ImageForm } from "@/components/image-form";
import { ImageList } from "@/components/image-list";
import { Folder, Plus, PanelLeft, Info } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Pool } from "@/lib/types";
import { ShareDialog } from "@/components/share-dialog";
import { RequireAuth } from "@/components/require-auth";

function PoolsContent() {
  const searchParams = useSearchParams();
  const poolId = searchParams.get("id");
  const [pools, setPools] = useState<Pool[]>([]);
  const [refreshKey, setRefreshKey] = useState(0);
  const [linkDialogOpen, setLinkDialogOpen] = useState(false);
  const [sizeIndex, setSizeIndex] = useState(2);

  const SIZES = [64, 96, 128, 192, 256];
  const SIZE_LABELS = ["-2", "-1", "0", "+1", "+2"];
  const maxHeight = SIZES[sizeIndex] || 128;

  const handleImageMoved = () => {
    setRefreshKey((k) => k + 1);
  };

  const activePool = pools.find(p => p.id === poolId);
  const isSubscribed = activePool?.is_subscribed;

  return (
    <div className="flex flex-1 min-h-0 w-full overflow-hidden rounded-xl bg-muted/30 border">
      {/* Sidebar Area */}
      <div className="w-64 shrink-0 hidden md:block">
        <PoolList onPoolsChange={setPools} onImageMoved={handleImageMoved} />
      </div>
      
      {/* Inset Main Content Area */}
      <div className="flex-1 flex flex-col m-2 rounded-xl bg-background shadow-sm border overflow-hidden">
        <header className="flex h-14 shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear">
          <div className="flex w-full items-center justify-between px-4 lg:px-6">
            <div className="flex items-center gap-2">
              <Sheet>
                <SheetTrigger render={<Button variant="ghost" size="icon" className="md:hidden h-8 w-8 -ml-2 text-muted-foreground"><PanelLeft className="h-5 w-5" /></Button>} />
                <SheetContent side="left" className="w-72 p-0 flex flex-col gap-0 border-r-0" showCloseButton={false}>
                  <SheetHeader className="sr-only">
                    <SheetTitle>Pools</SheetTitle>
                  </SheetHeader>
                  <PoolList isMobile onPoolsChange={setPools} onImageMoved={handleImageMoved} />
                </SheetContent>
              </Sheet>
              <h1 className="text-base font-medium flex items-center gap-2">
                {activePool ? activePool.name : "Images"}
                {activePool && (
                  <Dialog>
                    <DialogTrigger render={<Button variant="ghost" size="icon" className="h-6 w-6"><Info className="h-4 w-4 text-muted-foreground hover:text-foreground"/></Button>} />
                    <DialogContent className="sm:max-w-md">
                      <DialogHeader>
                        <DialogTitle>Pool Information</DialogTitle>
                      </DialogHeader>
                      <div className="space-y-4 text-sm mt-4">
                        <div className="flex justify-between border-b pb-2">
                          <span className="text-muted-foreground">Pool Name</span>
                          <span className="font-medium">{activePool.name}</span>
                        </div>
                        <div className="flex justify-between border-b pb-2">
                          <span className="text-muted-foreground">Owner</span>
                          <span className="font-medium">{activePool.owner_username || "Unknown"}</span>
                        </div>
                        <div className="flex justify-between border-b pb-2">
                          <span className="text-muted-foreground">Subscribers</span>
                          <span className="font-medium">{activePool.subscriber_count}</span>
                        </div>
                        <div className="flex justify-between border-b pb-2">
                          <span className="text-muted-foreground">Role</span>
                          <span className="font-medium">{activePool.is_subscribed ? "Subscriber" : "Owner"}</span>
                        </div>
                      </div>
                    </DialogContent>
                  </Dialog>
                )}
              </h1>
            </div>
            {poolId && !isSubscribed && activePool && (
              <div className="flex items-center gap-2">
                <Dialog>
                  <DialogTrigger render={<Button variant="outline" size="sm" className="h-8 gap-1" />}>
                    <span className="hidden sm:inline">Share Settings</span>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Share Pool</DialogTitle>
                    </DialogHeader>
                    <ShareDialog 
                      pool={activePool} 
                      onPoolChange={(updated) => setPools(pools.map(p => p.id === updated.id ? updated : p))}
                    />
                  </DialogContent>
                </Dialog>
                
                <Dialog open={linkDialogOpen} onOpenChange={setLinkDialogOpen}>
                  <DialogTrigger render={<Button size="sm" className="h-8 gap-1"><Plus className="h-4 w-4" /><span className="hidden sm:inline">Add Image</span></Button>} />
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Add New Image</DialogTitle>
                    </DialogHeader>
                    <ImageForm 
                      poolId={poolId} 
                      onCreated={() => { setRefreshKey((k) => k + 1); setLinkDialogOpen(false); }} 
                    />
                  </DialogContent>
                </Dialog>
              </div>
            )}
          </div>
        </header>
        {/* Size toolbar */}
        {poolId && (
          <div className="flex items-center gap-3 px-4 lg:px-6 h-10 shrink-0 border-b bg-muted/30">
            <span className="text-xs font-medium text-muted-foreground whitespace-nowrap">
              Size
            </span>
            <div className="relative flex items-center w-24 h-6">
              <div className="absolute left-0 right-0 h-0.5 rounded-full bg-border" />
              <div
                className="absolute left-0 h-0.5 rounded-full bg-primary transition-all duration-150"
                style={{ width: `${(sizeIndex / (SIZES.length - 1)) * 100}%` }}
              />
              {SIZES.map((_, i) => (
                <button
                  key={i}
                  type="button"
                  onClick={() => setSizeIndex(i)}
                  className="absolute flex items-center justify-center"
                  style={{ left: `${(i / (SIZES.length - 1)) * 100}%`, transform: "translateX(-50%)" }}
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
        )}
        <div className="flex-1 flex flex-col overflow-y-auto">
          {poolId ? (
            <div className="p-6 space-y-6 max-w-7xl mx-auto w-full">
              <ImageList key={`${poolId}:${refreshKey}`} poolId={poolId} maxHeight={maxHeight} readonly={isSubscribed} pools={pools} onMoveImage={handleImageMoved} />
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center animate-in fade-in-50">
              <div className="h-16 w-16 rounded-full bg-muted flex items-center justify-center mb-4">
                <Folder className="h-8 w-8 text-muted-foreground/50" />
              </div>
              <h2 className="text-xl font-semibold">Select a pool</h2>
              <p className="text-muted-foreground mt-2 max-w-sm">
                Choose a pool from the sidebar to view and manage its images.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function PoolsPage() {
  return (
    <AppShell>
      <Suspense fallback={<div className="flex items-center justify-center h-full min-h-screen">Loading...</div>}>
        <RequireAuth>
          <PoolsContent />
        </RequireAuth>
      </Suspense>
    </AppShell>
  );
}
