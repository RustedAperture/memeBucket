"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet } from "@/lib/api";

type Category = { id: string; name: string };

export function CategoryList() {
  const [categories, setCategories] = useState<Category[]>([]);

  async function load() {
    setCategories(await apiGet<Category[]>("/api/categories"));
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <div className="space-y-2">
      {categories.map((category) => (
        <div key={category.id} className="flex items-center justify-between border-b py-2">
          <Link href={`/categories/detail?id=${category.id}`} className="font-medium">{category.name}</Link>
          <Button
            variant="ghost"
            onClick={async () => {
              await apiDelete(`/api/categories/${category.id}`);
              await load();
            }}
          >
            Delete
          </Button>
        </div>
      ))}
    </div>
  );
}
