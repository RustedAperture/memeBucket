"use client";

import { AppShell } from "@/components/app-shell";
import { CategoryForm } from "@/components/category-form";
import { CategoryList } from "@/components/category-list";

export default function CategoriesPage() {
  return (
    <AppShell>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold">Categories</h1>
        <CategoryForm onCreated={() => window.location.reload()} />
        <CategoryList />
      </div>
    </AppShell>
  );
}
