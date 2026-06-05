"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";
import { Plus } from "lucide-react";

export function CategoryForm({ onCreated }: { onCreated: () => void }) {
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!name.trim()) return;
    setError(null);
    try {
      await apiPost("/api/categories", { name });
      setName("");
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not create category");
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-4 mt-2">
      <div className="flex flex-col gap-2">
        <Input 
          value={name} 
          onChange={(event) => setName(event.target.value)} 
          placeholder="New category name..." 
          className="w-full"
          autoFocus
        />
        {error ? <p className="text-sm font-medium text-destructive">{error}</p> : null}
      </div>
      <div className="flex justify-end">
        <Button type="submit">
          <Plus className="w-4 h-4 mr-2" />
          Add Category
        </Button>
      </div>
    </form>
  );
}
