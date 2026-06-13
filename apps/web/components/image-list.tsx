"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiDelete, apiGet, apiPatch, apiPost } from "@/lib/api";
import { Check, ExternalLink, ImageIcon, Trash2, Edit2, X, Star } from "lucide-react";
import { toast } from "sonner";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
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
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ImageItem, Pool } from "@/lib/types";

export function ImageList({ poolId, columnClass = "columns-2 sm:columns-2 md:columns-3 lg:columns-4", readonly = false, pools = [], onMoveImage }: { poolId: string; columnClass?: string; readonly?: boolean; pools?: Pool[]; onMoveImage?: () => void }) {
  const [images, setImages] = useState<ImageItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [imageToDelete, setImageToDelete] = useState<string | null>(null);
  const [selectedImage, setSelectedImage] = useState<ImageItem | null>(null);
  const [selectedImageIds, setSelectedImageIds] = useState<Set<string>>(new Set());
  const [editingMetadata, setEditingMetadata] = useState(false);
  const [titleValue, setTitleValue] = useState("");
  const [favoriteValue, setFavoriteValue] = useState(false);
  const [randomWeightValue, setRandomWeightValue] = useState(1);
  const [tagsValue, setTagsValue] = useState("");
  const [notesValue, setNotesValue] = useState("");
  const movePoolItems = pools.map((p) => ({
    label: `${p.name}${p.id === poolId ? " (Current)" : ""}`,
    value: p.id,
  }));

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
    setTitleValue(image.title || "");
    setFavoriteValue(image.favorite);
    setRandomWeightValue(image.randomWeight);
    setTagsValue(image.tags.join(", "));
    setNotesValue(image.notes || "");
    setEditingMetadata(false);
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

  async function handleSaveMetadata() {
    if (!selectedImage) return;
    const normalizedTags = parseTagInput(tagsValue);
    const normalizedTitle = titleValue.trim() || null;
    const normalizedNotes = notesValue.trim() || null;
    const normalizedWeight = clampRandomWeight(randomWeightValue);
    try {
      await apiPatch(`/api/pools/${poolId}/images/${selectedImage.id}`, {
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      });
      const updatedImage = {
        ...selectedImage,
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      };
      setSelectedImage(updatedImage);
      setImages(images.map(img => img.id === selectedImage.id ? updatedImage : img));
      setEditingMetadata(false);
    } catch {
      toast.error("Failed to save image details");
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
        <div className={`gap-4 ${columnClass}`}>
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
                className={`group relative overflow-hidden rounded-xl border transition-all cursor-pointer focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none break-inside-avoid mb-4 ${
                  isSelected
                    ? "border-primary ring-2 ring-primary"
                    : "border-border/70 hover:ring-2 hover:ring-ring"
                }`}
              >
                {image.url.split('?')[0].toLowerCase().endsWith('.mp4') || image.url.split('?')[0].toLowerCase().endsWith('.webm') ? (
                  <video
                    src={image.url}
                    autoPlay
                    loop
                    muted
                    playsInline
                    className="w-full h-auto object-cover block transition-transform duration-300 group-hover:scale-[1.02]"
                    onError={(e) => {
                      (e.target as HTMLVideoElement).style.display = 'none';
                      (e.target as HTMLVideoElement).nextElementSibling?.classList.remove('hidden');
                    }}
                  />
                ) : (
                  <img
                    src={image.url}
                    alt={image.title || "Image preview"}
                    className="w-full h-auto object-cover block transition-transform duration-300 group-hover:scale-[1.02]"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = 'none';
                      (e.target as HTMLImageElement).nextElementSibling?.classList.remove('hidden');
                    }}
                  />
                )}
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
          <DialogContent className="min-w-0 max-h-[calc(100dvh-2rem)] grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden sm:max-w-2xl">
            <DialogHeader>
              <DialogTitle className="truncate">{selectedImage.title || "Image details"}</DialogTitle>
              <DialogDescription>
                {formatAddedAt(selectedImage.createdAt)} - {selectedImage.sendCount} send{selectedImage.sendCount === 1 ? "" : "s"}
              </DialogDescription>
            </DialogHeader>

            <div className="grid min-w-0 min-h-0 grid-rows-[minmax(0,1fr)_auto_auto_auto] gap-4 overflow-y-auto pr-1">
              <div className="min-h-0 overflow-hidden rounded-xl border border-border/70 bg-muted/20">
                {selectedImage.url.split('?')[0].toLowerCase().endsWith('.mp4') || selectedImage.url.split('?')[0].toLowerCase().endsWith('.webm') ? (
                  <video
                    src={selectedImage.url}
                    autoPlay
                    loop
                    muted
                    playsInline
                    className="h-full max-h-full w-full object-contain"
                  />
                ) : (
                  <img
                    src={selectedImage.url}
                    alt={selectedImage.title || "Selected image preview"}
                    className="h-full max-h-full w-full object-contain"
                  />
                )}
              </div>

              <div className="space-y-3 rounded-lg border border-border/70 p-3">
                <div className="flex items-center justify-between gap-3">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Metadata</p>
                  {!readonly && !editingMetadata ? (
                    <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => setEditingMetadata(true)}>
                      <Edit2 className="h-3 w-3 mr-1" /> Edit
                    </Button>
                  ) : null}
                </div>

                {editingMetadata ? (
                  <div className="space-y-3">
                    <div className="space-y-1.5">
                      <Label htmlFor="image-title">Title</Label>
                      <Input
                        id="image-title"
                        value={titleValue}
                        maxLength={200}
                        onChange={(event) => setTitleValue(event.target.value)}
                        placeholder="Untitled"
                      />
                    </div>

                    <div className="grid gap-3 sm:grid-cols-[1fr_8rem]">
                      <div className="space-y-1.5">
                        <Label htmlFor="image-tags">Tags</Label>
                        <Input
                          id="image-tags"
                          value={tagsValue}
                          onChange={(event) => setTagsValue(event.target.value)}
                          placeholder="cat, reaction, happy"
                        />
                      </div>
                      <div className="space-y-1.5">
                        <Label htmlFor="image-weight">Weight</Label>
                        <Input
                          id="image-weight"
                          type="number"
                          min={0}
                          max={10}
                          step={1}
                          value={randomWeightValue}
                          onChange={(event) => setRandomWeightValue(clampRandomWeight(Number(event.target.value)))}
                        />
                      </div>
                    </div>

                    <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
                      <Label htmlFor="image-favorite" className="gap-2">
                        <Star className={favoriteValue ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
                        Favorite
                      </Label>
                      <Switch
                        id="image-favorite"
                        checked={favoriteValue}
                        onCheckedChange={setFavoriteValue}
                      />
                    </div>

                    <div className="space-y-1.5">
                      <Label htmlFor="image-notes">Notes / Credits</Label>
                      <Textarea
                        id="image-notes"
                        value={notesValue}
                        onChange={(event) => setNotesValue(event.target.value)}
                        placeholder="Add notes, credits, or context..."
                        className="resize-none h-24"
                      />
                    </div>

                    <div className="flex justify-end gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setEditingMetadata(false);
                          setTitleValue(selectedImage.title || "");
                          setFavoriteValue(selectedImage.favorite);
                          setRandomWeightValue(selectedImage.randomWeight);
                          setTagsValue(selectedImage.tags.join(", "));
                          setNotesValue(selectedImage.notes || "");
                        }}
                      >
                        Cancel
                      </Button>
                      <Button size="sm" onClick={handleSaveMetadata}>Save</Button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-3">
                    <div className="grid gap-2 text-sm sm:grid-cols-3">
                      <div>
                        <p className="text-xs text-muted-foreground">Favorite</p>
                        <p className="flex items-center gap-1 font-medium">
                          <Star className={selectedImage.favorite ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
                          {selectedImage.favorite ? "Yes" : "No"}
                        </p>
                      </div>
                      <div>
                        <p className="text-xs text-muted-foreground">Weight</p>
                        <p className="font-medium">{selectedImage.randomWeight}</p>
                      </div>
                      <div>
                        <p className="text-xs text-muted-foreground">Sends</p>
                        <p className="font-medium">{selectedImage.sendCount}</p>
                      </div>
                    </div>
                    <div className="space-y-1.5">
                      <p className="text-xs text-muted-foreground">Tags</p>
                      {selectedImage.tags.length > 0 ? (
                        <div className="flex flex-wrap gap-1.5">
                          {selectedImage.tags.map((tag) => (
                            <Badge key={tag} variant="secondary" className="max-w-full rounded-md">
                              <span className="truncate">{tag}</span>
                            </Badge>
                          ))}
                        </div>
                      ) : (
                        <p className="text-sm text-muted-foreground">No tags</p>
                      )}
                    </div>
                    <div className="space-y-1.5">
                      <p className="text-xs text-muted-foreground">Notes / Credits</p>
                      <p className="min-h-5 whitespace-pre-wrap text-sm text-foreground">
                        {selectedImage.notes || "No notes provided."}
                      </p>
                    </div>
                  </div>
                )}
              </div>

              <div className="space-y-2 min-w-0">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Link</p>
                <div className="flex min-w-0 gap-2">
                  <Input readOnly value={selectedImage.url} title={selectedImage.url} />
                  <Button
                    variant="secondary"
                    size="icon"
                    aria-label="Open image link"
                    render={<a href={selectedImage.url} target="_blank" rel="noreferrer" />}
                  >
                    <ExternalLink />
                  </Button>
                </div>
              </div>

              {pools.length > 1 && !readonly && (
                <div className="space-y-2">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Move to Pool</p>
                  <Select
                    items={movePoolItems}
                    value={poolId}
                    onValueChange={(newPoolId) => {
                      if (typeof newPoolId === "string") {
                        void handleMoveToPool(newPoolId);
                      }
                    }}
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectLabel>Pools</SelectLabel>
                        {pools.map((p) => (
                          <SelectItem
                            key={p.id}
                            value={p.id}
                            disabled={p.is_subscribed || p.id === poolId}
                          >
                            {p.name}{p.id === poolId ? " (Current)" : ""}
                          </SelectItem>
                        ))}
                      </SelectGroup>
                    </SelectContent>
                  </Select>
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

function clampRandomWeight(value: number) {
  if (!Number.isFinite(value)) {
    return 1;
  }
  return Math.min(10, Math.max(0, Math.round(value)));
}

function parseTagInput(value: string) {
  const tags: string[] = [];
  const seen = new Set<string>();

  for (const tag of value.split(",")) {
    const normalized = tag.trim().replace(/\s+/g, " ");
    if (!normalized) {
      continue;
    }
    const folded = normalized.toLowerCase();
    if (seen.has(folded)) {
      continue;
    }
    seen.add(folded);
    tags.push(normalized);
  }

  return tags;
}
