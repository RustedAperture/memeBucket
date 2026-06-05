"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet } from "@/lib/api";
import { ImageIcon, Trash2 } from "lucide-react";
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

type MediaLinkItem = { id: string; url: string };

export function MediaLinkList({ categoryId, maxHeight = 128 }: { categoryId: string; maxHeight?: number }) {
  const [links, setLinks] = useState<MediaLinkItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [linkToDelete, setLinkToDelete] = useState<string | null>(null);

  async function load() {
    try {
      setLinks(await apiGet<MediaLinkItem[]>(`/api/categories/${categoryId}/links`));
    } catch {
      // category might be empty or deleted
    }
  }

  useEffect(() => {
    void load();
  }, [categoryId]);

  async function handleDeleteConfirm() {
    if (!linkToDelete) return;
    setError(null);
    try {
      await apiDelete(`/api/categories/${categoryId}/links/${linkToDelete}`);
      setLinkToDelete(null);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not delete link");
    }
  }

  if (links.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center animate-in fade-in-50">
        <ImageIcon className="mx-auto h-10 w-10 text-muted-foreground/50" />
        <h2 className="mt-4 text-lg font-semibold">No links</h2>
        <p className="mb-4 mt-2 text-sm text-muted-foreground">No links in this category yet. Add one above.</p>
      </div>
    );
  }

  return (
    <>
      <div className="space-y-4">

        {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
        <div className="flex flex-wrap gap-4 items-start">
          {links.map((link) => (
            <div key={link.id} className="group relative overflow-hidden rounded-xl hover:ring-2 hover:ring-ring transition-all flex w-max">
              <img 
                src={link.url} 
                alt="Media preview" 
                style={{ maxHeight: `${maxHeight}px` }}
                className="w-auto object-cover block transition-transform duration-300 group-hover:scale-[1.02]"
                onError={(e) => {
                  (e.target as HTMLImageElement).style.display = 'none';
                  (e.target as HTMLImageElement).nextElementSibling?.classList.remove('hidden');
                }}
              />
              <ImageIcon className="hidden w-10 h-10 text-muted-foreground/50 absolute z-0" />
              
              <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none z-10" />
              
              <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 z-20">
                <Button variant="destructive" size="icon" className="h-8 w-8" onClick={(e) => { e.preventDefault(); setLinkToDelete(link.id); }}>
                  <Trash2 className="w-4 h-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </div>

      <AlertDialog open={!!linkToDelete} onOpenChange={(open) => !open && setLinkToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you sure?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete this link from your category. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleDeleteConfirm}>Delete</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
