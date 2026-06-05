"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiPost } from "@/lib/api";

export function CategoryForm({ onCreated }: { onCreated: () => void }) {
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
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
    <form onSubmit={submit} className="flex gap-2">
      <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="Category name" />
      <Button type="submit">Add</Button>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </form>
  );
}
