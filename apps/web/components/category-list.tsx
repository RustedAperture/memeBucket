"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet } from "@/lib/api";

type Category = { id: string; name: string };

export function CategoryList() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setCategories(await apiGet<Category[]>("/api/categories"));
  }

  useEffect(() => {
    void load();
  }, []);

  async function handleDelete(categoryId: string) {
    setError(null);
    try {
      await apiDelete(`/api/categories/${categoryId}`);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not delete category");
    }
  }

  return (
    <div className="space-y-2">
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {categories.map((category) => (
        <div key={category.id} className="flex items-center justify-between border-b py-2">
          <Link href={`/categories/detail?id=${category.id}`} className="font-medium">{category.name}</Link>
          <Button variant="ghost" onClick={() => handleDelete(category.id)}>
            Delete
          </Button>
        </div>
      ))}
    </div>
  );
}
