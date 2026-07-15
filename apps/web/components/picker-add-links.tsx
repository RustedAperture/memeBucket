"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, Inbox, Link2, TriangleAlert } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { apiPost } from "@/lib/api";
import { isWritablePickerBucket, parsePickerLinks, type PickerAddLinksSummary } from "@/lib/picker-add-links";
import type { Bucket } from "@/lib/types";

export type PickerAddLinksProps = {
  buckets: Bucket[];
  bucketId: string;
  onUseInbox: () => void;
  onBack: () => void;
  onSubmissionStateChange: (isSubmitting: boolean) => void;
};

export function PickerAddLinks({
  buckets,
  bucketId,
  onUseInbox,
  onBack,
  onSubmissionStateChange,
}: PickerAddLinksProps) {
  const [value, setValue] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [progress, setProgress] = useState<{ current: number; total: number } | null>(null);
  const [summary, setSummary] = useState<PickerAddLinksSummary | null>(null);
  const [validationMessage, setValidationMessage] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  const links = useMemo(() => parsePickerLinks(value), [value]);
  const isWritable = isWritablePickerBucket(bucketId, buckets);
  const isAllBuckets = bucketId === "all";
  const hasOwnedInbox = useMemo(
    () =>
      buckets.some(
        (bucket) =>
          bucket.name.trim().toLowerCase() === "inbox" &&
          !bucket.is_subscribed
      ),
    [buckets]
  );
  const submitDisabled = isSubmitting;

  const submitLabel = summary
    ? "Done"
    : isSubmitting && progress
      ? `Adding ${progress.current} of ${progress.total}…`
      : "Add links";

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (summary) {
      setSummary(null);
      setValidationMessage(null);
      return;
    }

    if (links.length === 0) {
      setValidationMessage("Enter at least one link.");
      return;
    }

    if (!isWritable) {
      setValidationMessage("Choose a destination bucket first.");
      return;
    }

    setIsSubmitting(true);
    onSubmissionStateChange(true);
    setValidationMessage(null);
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
    onSubmissionStateChange(false);
  }

  return (
    <form onSubmit={handleSubmit} className="flex h-full min-h-0 min-w-0 w-full flex-col gap-3 overflow-hidden p-2.5">
      <div className="flex items-center justify-between gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onBack} disabled={isSubmitting}>
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
        {hasOwnedInbox ? (
          <Button type="button" variant="outline" size="sm" onClick={onUseInbox} disabled={isSubmitting}>
            <Inbox className="h-4 w-4" />
            Use Inbox
          </Button>
        ) : null}
      </div>

      <div className="space-y-2">
        {isAllBuckets ? (
          <Alert variant="destructive" className="py-2">
            <TriangleAlert />
            <AlertDescription>
              Choose a specific bucket before adding links. All buckets is search-only here.
            </AlertDescription>
          </Alert>
        ) : !isWritable ? (
          <Alert variant="destructive" className="py-2">
            <TriangleAlert />
            <AlertDescription>This bucket can&apos;t accept new links from the picker.</AlertDescription>
          </Alert>
        ) : null}
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
        <label htmlFor="picker-add-links" className="text-xs font-medium text-muted-foreground">
          Links
        </label>
        {summary ? (
          <div className="flex min-h-36 min-w-0 flex-1 flex-col justify-center rounded-md border bg-card/60 px-3 py-4 text-sm">
            <p className="font-medium text-foreground">Finished adding links</p>
            <p className="mt-1 text-muted-foreground">
              Total: {summary.total} · Added: {summary.added} · Failed: {summary.failed}
            </p>
          </div>
        ) : (
          <>
            <Textarea
              id="picker-add-links"
              ref={textareaRef}
              value={value}
              onChange={(event) => {
                setValue(event.target.value);
                setValidationMessage(null);
              }}
              disabled={isSubmitting}
              placeholder={"https://example.com/one.gif\nhttps://example.com/two.mp4"}
              className="min-h-36 min-w-0 flex-1 resize-none overflow-x-hidden font-mono"
            />
            <p className="flex shrink-0 items-center gap-2 text-xs text-muted-foreground">
              <Link2 className="h-3.5 w-3.5 shrink-0" />
              Paste one link per line. They&apos;ll be added in order.
            </p>
          </>
        )}
      </div>

      {validationMessage ? (
        <p id="picker-add-links-validation" role="alert" className="text-xs text-destructive">
          {validationMessage}
        </p>
      ) : null}

      <Button
        type="submit"
        disabled={submitDisabled}
        aria-describedby={validationMessage ? "picker-add-links-validation" : undefined}
        className="w-full shrink-0"
      >
        {submitLabel}
      </Button>
    </form>
  );
}
