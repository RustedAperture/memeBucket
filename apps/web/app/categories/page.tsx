"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { AppShell } from "@/components/app-shell";
import { CategoryList } from "@/components/category-list";
import { MediaLinkForm } from "@/components/media-link-form";
import { MediaLinkList } from "@/components/media-link-list";
import { Folder, Plus, PanelLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

function CategoriesContent() {
  const searchParams = useSearchParams();
  const [categoryId, setCategoryId] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [linkDialogOpen, setLinkDialogOpen] = useState(false);
  const [sizeIndex, setSizeIndex] = useState(2);

  const SIZES = [64, 96, 128, 192, 256];
  const SIZE_LABELS = ["-2", "-1", "0", "+1", "+2"];
  const maxHeight = SIZES[sizeIndex] || 128;

  useEffect(() => {
    setCategoryId(searchParams.get("id"));
  }, [searchParams]);

  return (
    <div className="flex flex-1 min-h-0 w-full overflow-hidden rounded-xl bg-muted/30 border">
      {/* Sidebar Area */}
      <div className="w-64 shrink-0 hidden md:block">
        <CategoryList />
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
                    <SheetTitle>Categories</SheetTitle>
                  </SheetHeader>
                  <CategoryList isMobile />
                </SheetContent>
              </Sheet>
              <h1 className="text-base font-medium">Media Links</h1>
            </div>
            {categoryId && (
              <Dialog open={linkDialogOpen} onOpenChange={setLinkDialogOpen}>
                <DialogTrigger render={<Button size="sm" className="h-8 gap-1"><Plus className="h-4 w-4" /><span className="hidden sm:inline">Add Image</span></Button>} />
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Add New Image</DialogTitle>
                  </DialogHeader>
                  <MediaLinkForm 
                    categoryId={categoryId} 
                    onCreated={() => { setRefreshKey((k) => k + 1); setLinkDialogOpen(false); }} 
                  />
                </DialogContent>
              </Dialog>
            )}
          </div>
        </header>
        {/* Size toolbar */}
        {categoryId && (
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
          {categoryId ? (
            <div className="p-6 space-y-6 max-w-7xl mx-auto w-full">
              <MediaLinkList key={refreshKey} categoryId={categoryId} maxHeight={maxHeight} />
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center animate-in fade-in-50">
              <div className="h-16 w-16 rounded-full bg-muted flex items-center justify-center mb-4">
                <Folder className="h-8 w-8 text-muted-foreground/50" />
              </div>
              <h2 className="text-xl font-semibold">Select a category</h2>
              <p className="text-muted-foreground mt-2 max-w-sm">
                Choose a category from the sidebar to view and manage its media links.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function CategoriesPage() {
  return (
    <AppShell>
      <Suspense fallback={<div className="flex items-center justify-center h-full min-h-screen">Loading...</div>}>
        <CategoriesContent />
      </Suspense>
    </AppShell>
  );
}
