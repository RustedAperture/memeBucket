"use client";

import { useMemo, useState } from "react";
import { ArrowLeft, Folder, Inbox, Link2, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { apiPost } from "@/lib/api";
import { isWritablePickerBucket, parsePickerLinks, type PickerAddLinksSummary } from "@/lib/picker-add-links";
import type { Bucket } from "@/lib/types";

export type PickerAddLinksProps = {
  buckets: Bucket[];
  bucketId: string;
  onBucketChange: (bucketId: string) => void;
  onUseInbox: () => void;
  onBack: () => void;
};

export function PickerAddLinks({
  buckets,
  bucketId,
  onBucketChange,
  onUseInbox,
  onBack,
}: PickerAddLinksProps) {
  const [value, setValue] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [progress, setProgress] = useState<{ current: number; total: number } | null>(null);
  const [summary, setSummary] = useState<PickerAddLinksSummary | null>(null);

  const bucketItems = useMemo(
    () => [
      { label: "All buckets", value: "all" },
      ...buckets.map((bucket) => ({ label: bucket.name, value: bucket.id })),
    ],
    [buckets]
  );

  const links = useMemo(() => parsePickerLinks(value), [value]);
  const isWritable = isWritablePickerBucket(bucketId, buckets);
  const isAllBuckets = bucketId === "all";
  const hasOwnedInbox = useMemo(
    () =>
      buckets.some(
        (bucket) =>
          bucket.name.trim().toLowerCase() === "inbox" &&
          !bucket.is_subscribed &&
          !bucket.is_read_only
      ),
    [buckets]
  );
  const submitDisabled = isSubmitting || (!summary && (links.length === 0 || !isWritable));

  const submitLabel = summary
    ? "Done"
    : isSubmitting && progress
      ? `Adding ${progress.current} of ${progress.total}…`
      : "Add links";

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (summary) {
      setSummary(null);
      return;
    }

    if (links.length === 0 || !isWritable) {
      return;
    }

    setIsSubmitting(true);
    setProgress({ current: 1, total: links.length });

    let added = 0;
    for (let index = 0; index < links.length; index += 1) {
      const url = links[index];
      setProgress({ current: index + 1, total: links.length });
      try {
        await apiPost(`/api/buckets/${bucketId}/images`, { url });
        added += 1;
      } catch {
        // Keep going so the final summary reflects the whole batch.
      }
    }

    setSummary({ total: links.length, added, failed: links.length - added });
    setValue("");
    setProgress(null);
    setIsSubmitting(false);
  }

  return (
    <form onSubmit={handleSubmit} className="flex h-full flex-col gap-3 p-2.5">
      <div className="flex items-center justify-between gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
        {hasOwnedInbox ? (
          <Button type="button" variant="outline" size="sm" onClick={onUseInbox}>
            <Inbox className="h-4 w-4" />
            Use Inbox
          </Button>
        ) : null}
      </div>

      <div className="space-y-2">
        <label className="text-xs font-medium text-muted-foreground">Destination</label>
        <Select
          items={bucketItems}
          value={bucketId}
          onValueChange={(value) => {
            if (typeof value === "string") onBucketChange(value);
          }}
        >
          <SelectTrigger className="h-8 w-full justify-start rounded-md text-xs">
            <Folder className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="min-w-[220px]">
            <SelectGroup>
              <SelectLabel>Save to</SelectLabel>
              <SelectItem value="all">All buckets</SelectItem>
              {buckets.map((bucket) => (
                <SelectItem key={bucket.id} value={bucket.id}>
                  {bucket.name}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        {isAllBuckets ? (
          <p className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-2.5 py-2 text-xs text-amber-900 dark:text-amber-200">
            <TriangleAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            Choose a specific bucket before adding links. All buckets is search-only here.
          </p>
        ) : !isWritable ? (
          <p className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-2.5 py-2 text-xs text-amber-900 dark:text-amber-200">
            <TriangleAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            This bucket can&apos;t accept new links from the picker.
          </p>
        ) : null}
      </div>

      <div className="flex-1 space-y-2">
        <label htmlFor="picker-add-links" className="text-xs font-medium text-muted-foreground">
          Links
        </label>
        <Textarea
          id="picker-add-links"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          disabled={isSubmitting || summary !== null}
          placeholder={"https://example.com/one.gif\nhttps://example.com/two.mp4"}
          className="h-full min-h-36 resize-none rounded-md bg-background/60 font-mono text-xs"
        />
        <p className="flex items-center gap-2 text-xs text-muted-foreground">
          <Link2 className="h-3.5 w-3.5 shrink-0" />
          Paste one link per line. They&apos;ll be added in order.
        </p>
      </div>

      {summary ? (
        <div className="rounded-md border bg-card/60 px-3 py-2 text-xs">
          <p className="font-medium text-foreground">Finished adding links</p>
          <p className="mt-1 text-muted-foreground">
            Total: {summary.total} · Added: {summary.added} · Failed: {summary.failed}
          </p>
        </div>
      ) : null}

      <Button type="submit" disabled={submitDisabled} className="w-full rounded-md">
        {submitLabel}
      </Button>
    </form>
  );
}
