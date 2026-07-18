import type { Bucket } from "@/lib/types";

export type PickerAddLinksFailure = {
  url: string;
  error: string;
};

export type PickerAddLinksSummary = {
  total: number;
  added: number;
  failed: number;
  failedLinks: PickerAddLinksFailure[];
};

export function parsePickerLinks(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function isWritablePickerBucket(bucketId: string, buckets: Bucket[]): boolean {
  if (bucketId === "all") return false;
  const bucket = buckets.find((candidate) => candidate.id === bucketId);
  if (!bucket || bucket.is_subscribed) return false;

  const isOwnedInbox = bucket.name.trim().toLowerCase() === "inbox";
  return isOwnedInbox || !bucket.is_read_only;
}
