"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiDelete, apiGet, apiPatch, apiPost } from "@/lib/api";
import { ExternalLink, Info, Loader2, Tags, Trash2, Check, Download, Video, CheckSquare, Image as ImageIcon, Copy, Star, Edit2, X, HelpCircle, Ban } from "lucide-react";
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
import type { ImageItem, Bucket } from "@/lib/types";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

type BulkFavoriteValue = "unchanged" | "true" | "false";

export function ImageList({ bucketId, columnClass = "columns-2 sm:columns-2 md:columns-3 lg:columns-4", readonly = false, buckets = [], onMoveImage, onImageUpdated }: { bucketId: string; columnClass?: string; readonly?: boolean; buckets?: Bucket[]; onMoveImage?: () => void; onImageUpdated?: () => void }) {
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
  const [bulkDialogOpen, setBulkDialogOpen] = useState(false);
  const [bulkFavorite, setBulkFavorite] = useState<BulkFavoriteValue>("unchanged");
  const [bulkWeight, setBulkWeight] = useState("");
  const [bulkAddTags, setBulkAddTags] = useState("");
  const [bulkRemoveTags, setBulkRemoveTags] = useState("");
  const [bulkSaving, setBulkSaving] = useState(false);
  const moveBucketItems = buckets.map((p) => ({
    label: `${p.name}${p.id === bucketId ? " (Current)" : ""}`,
    value: p.id,
  }));

  async function load() {
    try {
      let loadedImages: ImageItem[] = [];
      if (bucketId === "favorites") {
        const results = await apiGet<any[]>(`/api/images/search?favorite=true&limit=1000`);
        loadedImages = results.map(r => ({ ...r.image, bucketId: r.bucketId }));
      } else {
        loadedImages = await apiGet<ImageItem[]>(`/api/buckets/${bucketId}/images`);
      }
      setImages(loadedImages);
      return loadedImages;
    } catch {
      // bucket might be empty or deleted
      return [];
    }
  }

  useEffect(() => {
    let cancelled = false;
    void Promise.resolve().then(() => {
      if (cancelled) {
        return;
      }
      setSelectedImage(null);
      setSelectedImageIds(new Set());
      setImageToDelete(null);
      setBulkDialogOpen(false);
      setEditingMetadata(false);
      resetBulkForm();
    });
    let request: Promise<any>;
    if (bucketId === "favorites") {
      request = apiGet<any[]>(`/api/images/search?favorite=true&limit=1000`)
        .then(results => results.map(r => ({ ...r.image, bucketId: r.bucketId })));
    } else {
      request = apiGet<ImageItem[]>(`/api/buckets/${bucketId}/images`);
    }

    request
      .then((loadedImages) => {
        if (!cancelled) {
          setImages(loadedImages);
        }
      })
      .catch(() => {
        // bucket might be empty or deleted
      });

    return () => {
      cancelled = true;
    };
  }, [bucketId]);

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
      await apiDelete(`/api/buckets/${bucketId}/images/${imageToDelete}`);
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
      await apiPatch(`/api/buckets/${bucketId}/images/${selectedImage.id}`, {
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      });
      const loadedImages = await load();
      const updatedImage = loadedImages.find((image) => image.id === selectedImage.id) ?? {
        ...selectedImage,
        title: normalizedTitle,
        notes: normalizedNotes,
        favorite: favoriteValue,
        randomWeight: normalizedWeight,
        tags: normalizedTags,
      };
      setSelectedImage(updatedImage);
      setEditingMetadata(false);
    } catch {
      toast.error("Failed to save image details");
    }
  }

  function resetBulkForm() {
    setBulkFavorite("unchanged");
    setBulkWeight("");
    setBulkAddTags("");
    setBulkRemoveTags("");
    setBulkSaving(false);
  }

  async function handleBulkSave() {
    const imageIds = Array.from(selectedImageIds);
    if (imageIds.length === 0) return;

    const addTags = parseTagInput(bulkAddTags);
    const removeTags = parseTagInput(bulkRemoveTags);
    const parsedWeight = parseOptionalRandomWeight(bulkWeight);
    if (!parsedWeight.ok) {
      toast.error("Weight must be a whole number between 0 and 10");
      return;
    }

    const payload: {
      imageIds: string[];
      favorite?: boolean;
      randomWeight?: number;
      addTags?: string[];
      removeTags?: string[];
    } = { imageIds };

    if (bulkFavorite !== "unchanged") {
      payload.favorite = bulkFavorite === "true";
    }
    if (parsedWeight.value !== undefined) {
      payload.randomWeight = parsedWeight.value;
    }
    if (addTags.length > 0) {
      payload.addTags = addTags;
    }
    if (removeTags.length > 0) {
      payload.removeTags = removeTags;
    }

    if (
      payload.favorite === undefined
      && payload.randomWeight === undefined
      && !payload.addTags
      && !payload.removeTags
    ) {
      toast.error("Choose at least one metadata change");
      return;
    }

    setBulkSaving(true);
    try {
      const response = await apiPatch<typeof payload, { updated: number }>(
        `/api/buckets/${bucketId}/images/bulk`,
        payload
      );
      setImages((currentImages) => applyBulkMetadataToImages(currentImages, imageIds, payload));
      setBulkDialogOpen(false);
      setSelectedImageIds(new Set());
      resetBulkForm();
      await load();
      toast.success(`Updated ${response.updated} image${response.updated === 1 ? "" : "s"}`);
      if (onImageUpdated) onImageUpdated();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to update images");
      setBulkSaving(false);
    }
  }

  async function handleMoveToBucket(newBucketId: string) {
    if (!selectedImage || newBucketId === bucketId) return;
    try {
      await apiPost(`/api/buckets/${bucketId}/images/${selectedImage.id}/move`, { new_bucket_id: newBucketId });
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
        <p className="mb-4 mt-2 text-sm text-muted-foreground">No images in this bucket yet. Add one above.</p>
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
            <div className="flex gap-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="h-7 px-2"
                onClick={() => setBulkDialogOpen(true)}
              >
                <Tags className="h-3.5 w-3.5" />
                Edit
              </Button>
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
                  event.dataTransfer.setData("application/json", JSON.stringify({ imageId: image.id, imageIds, sourceBucketId: bucketId }));
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

                <button
                  type="button"
                  aria-label={image.favorite ? "Remove from favorites" : "Add to favorites"}
                  onClick={async (event) => {
                    event.stopPropagation();
                    const newFav = !image.favorite;
                    // Optimistic update
                    setImages(prev => prev.map(img => img.id === image.id ? { ...img, favorite: newFav } : img));
                    try {
                      const patchBucketId = bucketId === "favorites" ? (image as any).bucketId : bucketId;
                      await apiPatch(`/api/buckets/${patchBucketId}/images/${image.id}`, { favorite: newFav });
                      if (onImageUpdated) onImageUpdated();
                    } catch (err) {
                      // Revert on failure
                      setImages(prev => prev.map(img => img.id === image.id ? { ...img, favorite: !newFav } : img));
                      toast.error(err instanceof Error ? err.message : "Failed to update favorite");
                    }
                  }}
                  className={`absolute right-2 top-2 z-20 flex h-7 w-7 items-center justify-center rounded-full transition-all ${
                    image.favorite
                      ? "text-yellow-400 bg-black/40 hover:scale-110 opacity-100"
                      : "text-white/70 bg-black/40 opacity-0 group-hover:opacity-100 focus:opacity-100 hover:scale-110 hover:text-white"
                  }`}
                >
                  <Star className="h-4 w-4" fill={image.favorite ? "currentColor" : "none"} />
                </button>

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

      <Dialog
        open={bulkDialogOpen}
        onOpenChange={(open) => {
          setBulkDialogOpen(open);
          if (!open) {
            resetBulkForm();
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Bulk edit</DialogTitle>
            <DialogDescription>
              {selectedImageIds.size} image{selectedImageIds.size === 1 ? "" : "s"} selected
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label>Favorite</Label>
              <Select
                items={[
                  { label: "No change", value: "unchanged" },
                  { label: "Favorite", value: "true" },
                  { label: "Not favorite", value: "false" },
                ]}
                value={bulkFavorite}
                onValueChange={(value) => {
                  if (value === "unchanged" || value === "true" || value === "false") {
                    setBulkFavorite(value);
                  }
                }}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem value="unchanged">No change</SelectItem>
                    <SelectItem value="true">Favorite</SelectItem>
                    <SelectItem value="false">Not favorite</SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-1.5">
                <Label htmlFor="bulk-weight">Weight</Label>
                <Tooltip>
                  <TooltipTrigger className="cursor-help outline-none p-0 bg-transparent border-none inline-flex items-center justify-center">
                    <HelpCircle className="h-4 w-4 text-muted-foreground hover:text-foreground transition-colors" />
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs">
                    <div className="flex flex-col gap-1.5">
                      <p>Weight (0-10) determines how likely this image is to be picked randomly.</p>
                      <ul className="list-disc pl-4 opacity-90">
                        <li><strong>0</strong>: Disabled (never picked)</li>
                        <li><strong>1-10</strong>: Higher = more likely</li>
                      </ul>
                    </div>
                  </TooltipContent>
                </Tooltip>
              </div>
              <Input
                id="bulk-weight"
                type="number"
                min={0}
                max={10}
                step={1}
                value={bulkWeight}
                onChange={(event) => setBulkWeight(event.target.value)}
                placeholder="No change"
              />
            </div>

            <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
              <Label htmlFor="bulk-weight-disable" className="flex items-center gap-2 cursor-pointer">
                <Ban className={bulkWeight === "0" ? "h-4 w-4 text-destructive" : "h-4 w-4 text-muted-foreground"} />
                Disable usage
              </Label>
              <Switch
                id="bulk-weight-disable"
                checked={bulkWeight === "0"}
                onCheckedChange={(checked) => setBulkWeight(checked ? "0" : "")}
                className="data-[state=checked]:bg-destructive"
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="bulk-add-tags">Add tags</Label>
              <Input
                id="bulk-add-tags"
                value={bulkAddTags}
                onChange={(event) => setBulkAddTags(event.target.value)}
                placeholder="cat, reaction"
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="bulk-remove-tags">Remove tags</Label>
              <Input
                id="bulk-remove-tags"
                value={bulkRemoveTags}
                onChange={(event) => setBulkRemoveTags(event.target.value)}
                placeholder="old, duplicate"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setBulkDialogOpen(false)}
              disabled={bulkSaving}
            >
              Cancel
            </Button>
            <Button onClick={handleBulkSave} disabled={bulkSaving}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
                        <div className="flex items-center gap-1.5">
                          <Label htmlFor="image-weight">Weight</Label>
                          <Tooltip>
                            <TooltipTrigger className="cursor-help outline-none p-0 bg-transparent border-none inline-flex items-center justify-center">
                              <HelpCircle className="h-4 w-4 text-muted-foreground hover:text-foreground transition-colors" />
                            </TooltipTrigger>
                            <TooltipContent className="max-w-xs">
                              <div className="flex flex-col gap-1.5">
                                <p>Weight (0-10) determines how likely this image is to be picked randomly.</p>
                                <ul className="list-disc pl-4 opacity-90">
                                  <li><strong>0</strong>: Disabled (never picked)</li>
                                  <li><strong>1-10</strong>: Higher = more likely</li>
                                </ul>
                              </div>
                            </TooltipContent>
                          </Tooltip>
                        </div>
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

                    <div className="grid gap-3 sm:grid-cols-2">
                      <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
                        <Label htmlFor="image-favorite" className="flex items-center gap-2 cursor-pointer">
                          <Star className={favoriteValue ? "h-4 w-4 fill-current text-primary" : "h-4 w-4 text-muted-foreground"} />
                          Favorite
                        </Label>
                        <Switch
                          id="image-favorite"
                          checked={favoriteValue}
                          onCheckedChange={setFavoriteValue}
                        />
                      </div>
                      
                      <div className="flex items-center justify-between rounded-md border bg-muted/30 px-3 py-2">
                        <Label htmlFor="image-weight-disable" className="flex items-center gap-2 cursor-pointer">
                          <Ban className={randomWeightValue === 0 ? "h-4 w-4 text-destructive" : "h-4 w-4 text-muted-foreground"} />
                          Disable usage
                        </Label>
                        <Switch
                          id="image-weight-disable"
                          checked={randomWeightValue === 0}
                          onCheckedChange={(checked) => setRandomWeightValue(checked ? 0 : 1)}
                          className="data-[state=checked]:bg-destructive"
                        />
                      </div>
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

              {buckets.length > 1 && !readonly && (
                <div className="space-y-2">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Move to Bucket</p>
                  <Select
                    items={moveBucketItems}
                    value={bucketId}
                    onValueChange={(newBucketId) => {
                      if (typeof newBucketId === "string") {
                        void handleMoveToBucket(newBucketId);
                      }
                    }}
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectLabel>Buckets</SelectLabel>
                        {buckets.map((p) => (
                          <SelectItem
                            key={p.id}
                            value={p.id}
                            disabled={p.is_subscribed || p.id === bucketId}
                          >
                            {p.name}{p.id === bucketId ? " (Current)" : ""}
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
              This will permanently delete this image from your bucket. This action cannot be undone.
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
    const normalized = normalizeTag(tag);
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

function normalizeTag(value: string) {
  const normalized = value
    .trim()
    .replace(/^[^A-Za-z0-9_-]+|[^A-Za-z0-9_-]+$/g, "")
    .replace(/\s+/g, " ");

  return normalized || null;
}

function parseOptionalRandomWeight(value: string): { ok: true; value?: number } | { ok: false } {
  const trimmed = value.trim();
  if (!trimmed) {
    return { ok: true };
  }
  if (!/^(?:[0-9]|10)$/.test(trimmed)) {
    return { ok: false };
  }
  return { ok: true, value: Number(trimmed) };
}

function applyBulkMetadataToImages(
  images: ImageItem[],
  imageIds: string[],
  payload: {
    favorite?: boolean;
    randomWeight?: number;
    addTags?: string[];
    removeTags?: string[];
  }
) {
  const selectedIds = new Set(imageIds);
  const removeTagFolds = new Set((payload.removeTags ?? []).map((tag) => tag.toLowerCase()));

  return images.map((image) => {
    if (!selectedIds.has(image.id)) {
      return image;
    }

    let tags = image.tags.filter((tag) => !removeTagFolds.has(tag.toLowerCase()));
    const seenTagFolds = new Set(tags.map((tag) => tag.toLowerCase()));
    for (const tag of payload.addTags ?? []) {
      const folded = tag.toLowerCase();
      if (!seenTagFolds.has(folded)) {
        tags = [...tags, tag];
        seenTagFolds.add(folded);
      }
    }

    return {
      ...image,
      favorite: payload.favorite ?? image.favorite,
      randomWeight: payload.randomWeight ?? image.randomWeight,
      tags,
    };
  });
}
