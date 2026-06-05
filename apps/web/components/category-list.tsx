"use client";

import { Folder, Plus, Trash2, X } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { apiDelete, apiGet } from "@/lib/api";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { CategoryForm } from "@/components/category-form";
import { SheetClose } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";

type Category = { id: string; name: string };

export function CategoryList({ isMobile }: { isMobile?: boolean }) {
  const [categories, setCategories] = useState<Category[]>([]);
  const [error, setError] = useState<string | null>(null);
  const searchParams = useSearchParams();
  const activeId = searchParams.get("id");
  const [dialogOpen, setDialogOpen] = useState(false);

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
    <div className="flex flex-col h-full bg-sidebar/50 text-sidebar-foreground">
      <div className="flex h-14 items-center justify-between pl-4 pr-2">
        <span className="font-semibold text-lg">Folders</span>
        
        <div className={cn("flex items-center", isMobile && "-space-x-px")}>
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger render={<Button variant="outline" size="icon" className={cn("h-8 w-8 relative z-10", isMobile && "rounded-r-none hover:z-20")} />}>
              <Plus className="w-4 h-4" />
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Add New Category</DialogTitle>
              </DialogHeader>
              <CategoryForm onCreated={() => { setDialogOpen(false); void load(); }} />
            </DialogContent>
          </Dialog>
          {isMobile && (
            <SheetClose render={<Button variant="outline" size="icon" className="h-8 w-8 rounded-l-none relative z-10 hover:z-20" />}>
              <X className="w-4 h-4" />
            </SheetClose>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-4">
        <div className="mb-2 px-2 text-xs font-medium text-sidebar-foreground/70">Your Collections</div>
        {error ? <p className="text-sm font-medium text-destructive px-2 pb-2">{error}</p> : null}
        {categories.length === 0 ? (
          <p className="text-sm text-muted-foreground p-4 text-center">No categories yet.</p>
        ) : (
          <div className="space-y-1">
            {categories.map((category) => {
              const isActive = category.id === activeId;
              return (
                <div key={category.id} className="group relative">
                  <Link 
                    href={`/categories?id=${category.id}`} 
                    className={`flex h-8 items-center gap-2 overflow-hidden rounded-md px-2 text-sm ring-sidebar-ring outline-hidden transition-all ${
                      isActive 
                        ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground" 
                        : "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                    }`}
                  >
                    <Folder className="h-4 w-4 shrink-0" />
                    <span className="truncate flex-1">{category.name}</span>
                  </Link>
                  <Button 
                    variant="ghost" 
                    size="icon" 
                    className={`absolute right-1 top-1 h-6 w-6 rounded-md p-0 opacity-0 transition-opacity hover:bg-muted focus-visible:opacity-100 group-hover:opacity-100 ${
                      isActive ? "text-sidebar-accent-foreground" : "text-muted-foreground"
                    }`}
                    onClick={(e) => { e.preventDefault(); handleDelete(category.id); }}
                  >
                    <Trash2 className="w-3 h-3 text-destructive" />
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
