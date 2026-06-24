"use client";

import { useEffect, useState, Suspense } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { apiGet, apiPost } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { AppShell } from "@/components/app-shell";

function ShareContent() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token");
  const [bucket, setBucket] = useState<{ id: string; name: string; subscriber_count: number; images: { id: string; url: string }[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [subscribing, setSubscribing] = useState(false);
  const router = useRouter();

  useEffect(() => {
    if (!token) return;
    apiGet<{ id: string; name: string; subscriber_count: number; images: { id: string; url: string }[] }>(`/api/share/${token}`)
      .then(setBucket)
      .catch((err) => setError(err.message));
  }, [token]);

  async function handleSubscribe() {
    if (!token) return;
    setSubscribing(true);
    try {
      await apiPost(`/api/share/${token}/subscribe`, {});
      router.push(`/buckets?id=${bucket?.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to subscribe");
      setSubscribing(false);
    }
  }

  if (!token) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8 text-center bg-background min-h-[50vh] rounded-xl border m-2 shadow-sm">
        <div className="text-destructive font-medium">Invalid share link.</div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8 text-center bg-background min-h-[50vh] rounded-xl border m-2 shadow-sm">
      {error ? (
        <div className="text-destructive font-medium">{error}</div>
      ) : bucket ? (
        <div className="space-y-8 max-w-4xl w-full animate-in fade-in zoom-in-95">
          <div className="space-y-2 flex flex-col items-center">
            <h1 className="text-2xl font-bold tracking-tight text-center">Subscribe to Bucket</h1>
            <p className="text-muted-foreground text-lg flex items-center justify-center gap-2 text-center">
              &quot;{bucket.name}&quot;
              <span className="text-sm bg-muted px-2 py-0.5 rounded-full flex items-center justify-center gap-1">
                {bucket.subscriber_count} subscriber{bucket.subscriber_count !== 1 ? 's' : ''}
              </span>
            </p>
          </div>
          
          <div className="p-6 bg-muted/50 rounded-xl border space-y-4 max-w-md mx-auto">
            <p className="text-sm text-muted-foreground text-center">
              Subscribing to this bucket will add it to your Discord <code className="bg-muted px-1.5 py-0.5 rounded">/ez</code> command autocomplete. You will be able to send random images from this bucket, but you cannot add or remove images yourself.
            </p>
            
            <Button 
              onClick={handleSubscribe} 
              disabled={subscribing}
              className="w-full"
              size="lg"
            >
              {subscribing ? "Subscribing..." : "Subscribe to Bucket"}
            </Button>
          </div>

          {bucket.images && bucket.images.length > 0 && (
            <div className="space-y-4 text-center mt-8">
              <h2 className="text-xl font-semibold">Preview Images</h2>
              <div className="flex flex-wrap gap-4 items-center justify-center">
                {bucket.images.slice(0, 50).map((image) => (
                  <div
                    key={image.id}
                    className="relative overflow-hidden rounded-xl border border-border/70 flex w-max"
                  >
                    <img 
                      src={image.url} 
                      alt="Image preview" 
                      style={{ maxHeight: '128px' }}
                      className="w-auto object-cover block"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = 'none';
                      }}
                    />
                  </div>
                ))}
                {bucket.images.length > 50 && (
                  <div className="flex items-center justify-center h-[128px] px-4 rounded-xl border border-dashed border-border/70 text-muted-foreground">
                    + {bucket.images.length - 50} more
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="text-muted-foreground animate-pulse">Loading bucket details...</div>
      )}
    </div>
  );
}

export default function SharePage() {
  return (
    <AppShell>
      <Suspense fallback={<div className="flex items-center justify-center h-full min-h-screen">Loading...</div>}>
        <ShareContent />
      </Suspense>
    </AppShell>
  );
}
