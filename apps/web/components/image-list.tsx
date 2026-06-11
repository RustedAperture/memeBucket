"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet, apiPatch, apiPost } from "@/lib/api";
import { Check, ExternalLink, ImageIcon, Trash2, Edit2, X } from "lucide-react";
import { toast } from "sonner";
import { Textarea } from "@/components/ui/textarea";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

type ImageItem = { id: string; url: string; createdAt?: string; notes?: string | null };
import { Pool } from "@/lib/types";

export function ImageList({ poolId, maxHeight = 128, readonly = false, pools = [], onMoveImage }: { poolId: string; maxHeight?: number; readonly?: boolean; pools?: Pool[]; onMoveImage?: () => void }) {
  const [images, setImages] = useState<ImageItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [imageToDelete, setImageToDelete] = useState<string | null>(null);
  const [selectedImage, setSelectedImage] = useState<ImageItem | null>(null);
  const [selectedImageIds, setSelectedImageIds] = useState<Set<string>>(new Set());
  const [editingNotes, setEditingNotes] = useState(false);
  const [notesValue, setNotesValue] = useState("");

  async function load() {
    try {
      setImages(await apiGet<ImageItem[]>(`/api/pools/${poolId}/images`));
    } catch {
      // pool might be empty or deleted
    }
  }

  useEffect(() => {
    let cancelled = false;
    void apiGet<ImageItem[]>(`/api/pools/${poolId}/images`)
      .then((loadedImages) => {
        if (!cancelled) {
          setImages(loadedImages);
        }
      })
      .catch(() => {
        // pool might be empty or deleted
      });

    return () => {
      cancelled = true;
    };
  }, [poolId]);

  function toggleImageSelection(imageId: string) {
    setSelectedImageIds((current) => {
      const next = new Set(current);
      if (next.has(imageId)) {
        next.delete(imageId);
      } else {
        next.add(imageId);
      }
      return next;
    });
  }

  function openImageDetails(image: ImageItem) {
    setSelectedImage(image);
    setNotesValue(image.notes || "");
    setEditingNotes(false);
  }

  function dragImageIdsFor(imageId: string) {
    return selectedImageIds.has(imageId) ? Array.from(selectedImageIds) : [imageId];
  }

  async function handleDeleteConfirm() {
    if (!imageToDelete) return;
    setError(null);
    try {
      await apiDelete(`/api/pools/${poolId}/images/${imageToDelete}`);
      if (selectedImage?.id === imageToDelete) {
        setSelectedImage(null);
      }
      setImageToDelete(null);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not delete image");
    }
  }

  async function handleSaveNotes() {
    if (!selectedImage) return;
    try {
      await apiPatch(`/api/pools/${poolId}/images/${selectedImage.id}`, { notes: notesValue });
      const updatedNotes = notesValue.trim() || null;
      setSelectedImage({ ...selectedImage, notes: updatedNotes });
      setImages(images.map(img => img.id === selectedImage.id ? { ...img, notes: updatedNotes } : img));
      setEditingNotes(false);
    } catch {
      toast.error("Failed to save notes");
    }
  }

  async function handleMoveToPool(newPoolId: string) {
    if (!selectedImage || newPoolId === poolId) return;
    try {
      await apiPost(`/api/pools/${poolId}/images/${selectedImage.id}/move`, { new_pool_id: newPoolId });
      setSelectedImage(null);
      if (onMoveImage) {
        onMoveImage();
      } else {
        await load();
      }
      toast.success("Image moved successfully");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to move image");
    }
  }

  if (images.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center animate-in fade-in-50">
        <ImageIcon className="mx-auto h-10 w-10 text-muted-foreground/50" />
        <h2 className="mt-4 text-lg font-semibold">No images</h2>
        <p className="mb-4 mt-2 text-sm text-muted-foreground">No images in this pool yet. Add one above.</p>
      </div>
    );
  }

  return (
    <>
      <div className="space-y-4">

        {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
        {!readonly && selectedImageIds.size > 0 ? (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-muted/40 px-3 py-2 text-sm">
            <span className="font-medium">
              {selectedImageIds.size} image{selectedImageIds.size === 1 ? "" : "s"} selected
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2"
              onClick={() => setSelectedImageIds(new Set())}
            >
              <X className="h-3.5 w-3.5" />
              Clear
            </Button>
          </div>
        ) : null}
        <div className="flex flex-wrap gap-4 items-start">
          {images.map((image) => {
            const isSelected = selectedImageIds.has(image.id);
            const dragCount = dragImageIdsFor(image.id).length;

            return (
              <div
                key={image.id}
                role="button"
                tabIndex={0}
                aria-pressed={isSelected}
                onClick={(event) => {
                  if (!readonly && (event.metaKey || event.ctrlKey || event.shiftKey)) {
                    toggleImageSelection(image.id);
                    return;
                  }
                  openImageDetails(image);
                }}
                onKeyDown={(event) => {
                  if (!readonly && event.key === " ") {
                    event.preventDefault();
                    toggleImageSelection(image.id);
                    return;
                  }
                  if (event.key === "Enter") {
                    event.preventDefault();
                    openImageDetails(image);
                  }
                }}
                draggable={!readonly}
                onDragStart={(event) => {
                  const imageIds = dragImageIdsFor(image.id);
                  event.dataTransfer.setData("application/json", JSON.stringify({ imageId: image.id, imageIds, sourcePoolId: poolId }));
                  event.dataTransfer.effectAllowed = "move";
                }}
                className={`group relative overflow-hidden rounded-xl border transition-all flex w-max cursor-pointer focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none ${
                  isSelected
                    ? "border-primary ring-2 ring-primary"
                    : "border-border/70 hover:ring-2 hover:ring-ring"
                }`}
              >
                <img 
                  src={image.url} 
                  alt="Image preview" 
                  style={{ maxHeight: `${maxHeight}px` }}
                  className="w-auto object-cover block transition-transform duration-300 group-hover:scale-[1.02]"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = 'none';
                    (e.target as HTMLImageElement).nextElementSibling?.classList.remove('hidden');
                  }}
                />
                <ImageIcon className="hidden w-10 h-10 text-muted-foreground/50 absolute z-0" />
                
                <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none z-10" />

                {!readonly ? (
                  <button
                    type="button"
                    aria-label={isSelected ? "Deselect image" : "Select image"}
                    aria-pressed={isSelected}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleImageSelection(image.id);
                    }}
                    className={`absolute left-2 top-2 z-20 flex h-6 w-6 items-center justify-center rounded-full border text-xs transition-opacity ${
                      isSelected
                        ? "border-primary bg-primary text-primary-foreground opacity-100"
                        : "border-white/70 bg-black/45 text-white opacity-0 group-hover:opacity-100 focus:opacity-100"
                    }`}
                  >
                    {isSelected ? <Check className="h-3.5 w-3.5" /> : null}
                  </button>
                ) : null}

                {!readonly && dragCount > 1 ? (
                  <div className="absolute bottom-2 right-2 z-20 rounded-full bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground">
                    {dragCount}
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>

      <Dialog open={!!selectedImage} onOpenChange={(open) => !open && setSelectedImage(null)}>
        {selectedImage ? (
          <DialogContent className="sm:max-w-2xl">
            <DialogHeader>
              <DialogTitle>Image details</DialogTitle>
              <DialogDescription>
                {formatAddedAt(selectedImage.createdAt)}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="overflow-hidden rounded-xl border border-border/70 bg-muted/20">
                <img
                  src={selectedImage.url}
                  alt="Selected image preview"
                  className="max-h-[60vh] w-full object-contain"
                />
              </div>

              <div className="space-y-2">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Link</p>
                <a
                  href={selectedImage.url}
                  target="_blank"
                  rel="noreferrer"
                  className="flex items-center gap-2 rounded-lg border bg-secondary/40 px-3 py-2 text-sm text-foreground transition-colors hover:bg-secondary"
                >
                  <span className="min-w-0 flex-1 truncate">{selectedImage.url}</span>
                  <ExternalLink className="h-4 w-4 shrink-0 text-muted-foreground" />
                </a>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Notes / Credits</p>
                  {!readonly && !editingNotes && (
                    <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => setEditingNotes(true)}>
                      <Edit2 className="h-3 w-3 mr-1" /> Edit
                    </Button>
                  )}
                </div>
                {editingNotes ? (
                  <div className="space-y-2">
                    <Textarea 
                      value={notesValue}
                      onChange={(e) => setNotesValue(e.target.value)}
                      placeholder="Add notes, credits, or context..."
                      className="resize-none h-24"
                    />
                    <div className="flex justify-end gap-2">
                      <Button variant="outline" size="sm" onClick={() => { setEditingNotes(false); setNotesValue(selectedImage.notes || ""); }}>Cancel</Button>
                      <Button size="sm" onClick={handleSaveNotes}>Save</Button>
                    </div>
                  </div>
                ) : (
                  <div className="rounded-lg border bg-secondary/20 px-3 py-2 text-sm text-foreground min-h-[2.5rem] whitespace-pre-wrap">
                    {selectedImage.notes ? selectedImage.notes : <span className="text-muted-foreground italic">No notes provided.</span>}
                  </div>
                )}
              </div>

              {pools.length > 1 && !readonly && (
                <div className="space-y-2">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Move to Pool</p>
                  <select 
                    className="flex h-9 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    value={poolId}
                    onChange={(e) => handleMoveToPool(e.target.value)}
                  >
                    {pools.map(p => (
                      <option key={p.id} value={p.id} disabled={p.is_subscribed || p.id === poolId}>
                        {p.name} {p.id === poolId ? "(Current)" : ""}
                      </option>
                    ))}
                  </select>
                </div>
              )}
            </div>

            {!readonly && (
              <DialogFooter>
                <Button
                  variant="destructive"
                  onClick={() => setImageToDelete(selectedImage.id)}
                >
                  <Trash2 className="w-4 h-4 mr-2" />
                  Delete image
                </Button>
              </DialogFooter>
            )}
          </DialogContent>
        ) : null}
      </Dialog>

      <AlertDialog open={!!imageToDelete} onOpenChange={(open) => !open && setImageToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you sure?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete this image from your pool. This action cannot be undone.
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

function formatAddedAt(value?: string) {
  if (!value) {
    return "Added date unavailable";
  }

  const date = new Date(value.endsWith("Z") ? value : `${value}Z`);
  if (Number.isNaN(date.getTime())) {
    return `Added ${value}`;
  }

  return `Added ${new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date)}`;
}
