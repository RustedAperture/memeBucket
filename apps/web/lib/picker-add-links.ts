import type { Bucket } from "@/lib/types";

export type PickerAddLinksSummary = {
  total: number;
  added: number;
  failed: number;
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
  return Boolean(bucket && !bucket.is_subscribed && !bucket.is_read_only);
}
