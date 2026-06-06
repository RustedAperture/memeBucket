"use client";

import { useState } from "react";
import { useUser } from "./user-provider";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./ui/dialog";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { apiPost } from "@/lib/api";
import { User } from "@/lib/types";

export function UsernameModal() {
  const { user, loading, refreshUser } = useUser();
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Show if finished loading, user is logged in, but has no username
  const isOpen = !loading && user !== null && user.username === null;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!username.trim()) return;
    
    setSubmitting(true);
    setError(null);
    try {
      // Using PATCH /api/account/username
      const res = await fetch("/api/account/username", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username: username.trim() }),
      });
      
      if (!res.ok) {
        const errorData = await res.json().catch(() => null);
        throw new Error(errorData?.error || "Failed to set username");
      }
      
      await refreshUser();
    } catch (err) {
      setError(err instanceof Error ? err.message : "An error occurred");
    } finally {
      setSubmitting(false);
    }
  }

  // Prevent closing by not providing onOpenChange or preventing default
  return (
    <Dialog open={isOpen}>
      <DialogContent className="sm:max-w-md" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Set your username</DialogTitle>
          <DialogDescription>
            Please choose a unique username to continue. This will be used when collaborating on shared pools.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="flex flex-col gap-4 mt-4">
          <div className="flex flex-col gap-2">
            <Input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="e.g. cool_user_123"
              disabled={submitting}
              autoFocus
            />
            {error && <p className="text-sm font-medium text-destructive">{error}</p>}
          </div>
          <Button type="submit" disabled={submitting || !username.trim()}>
            Save Username
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
