"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import { useRouter } from "next/navigation";
import { Plus } from "lucide-react";

export function PoolForm({ onCreated }: { onCreated: () => void }) {
  const [mode, setMode] = useState<"create" | "join">("create");
  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  async function submitCreate(event: React.FormEvent) {
    event.preventDefault();
    if (!name.trim()) return;
    setError(null);
    try {
      await apiPost("/api/pools", { name });
      setName("");
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not create pool");
    }
  }

  async function submitJoin(event: React.FormEvent) {
    event.preventDefault();
    if (!code.trim()) return;
    router.push(`/share?token=${code.trim()}`);
    // Optional: close the dialog, but router.push will navigate away anyway
    onCreated();
  }

  return (
    <div className="flex flex-col gap-4 mt-2">
      <div className="flex bg-muted p-1 rounded-md">
        <button
          type="button"
          onClick={() => { setMode("create"); setError(null); }}
          className={`flex-1 text-sm font-medium py-1.5 rounded-sm transition-all ${mode === "create" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
        >
          Create New
        </button>
        <button
          type="button"
          onClick={() => { setMode("join"); setError(null); }}
          className={`flex-1 text-sm font-medium py-1.5 rounded-sm transition-all ${mode === "join" ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
        >
          Join Existing
        </button>
      </div>

      {mode === "create" ? (
        <form onSubmit={submitCreate} className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Input 
              value={name} 
              onChange={(event) => setName(event.target.value)} 
              placeholder="New pool name..." 
              className="w-full"
              autoFocus
            />
            {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
          </div>
          <div className="flex justify-end">
            <Button type="submit">
              <Plus className="w-4 h-4 mr-2" />
              Create Pool
            </Button>
          </div>
        </form>
      ) : (
        <form onSubmit={submitJoin} className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Input 
              value={code} 
              onChange={(event) => setCode(event.target.value)} 
              placeholder="Enter share code (e.g. xY7b9P)..." 
              className="w-full"
              autoFocus
            />
            {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
          </div>
          <div className="flex justify-end">
            <Button type="submit">
              Preview Pool
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}
