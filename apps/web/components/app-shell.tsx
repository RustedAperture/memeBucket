"use client";

import Link from "next/link";
import { LayoutDashboard, FolderOpen } from "lucide-react";
import { AccountModal } from "./account-modal";
import { useUser } from "./user-provider";

export function AppShell({ children }: { children: React.ReactNode }) {
  const { user } = useUser();
  
  return (
    <div className="min-h-screen flex flex-col bg-background text-foreground">
      <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container mx-auto flex h-14 max-w-5xl items-center justify-between px-4">
          <Link href="/" className="font-semibold text-lg hover:opacity-80 transition-opacity">
            ezGif
          </Link>
          <nav className="flex items-center gap-6 text-sm font-medium">
            <Link href="/" className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors">
              <LayoutDashboard className="w-4 h-4" />
              <span className="hidden sm:inline">Dashboard</span>
            </Link>
            <Link href="/pools" className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors">
              <FolderOpen className="w-4 h-4" />
              <span className="hidden sm:inline">Pools</span>
            </Link>
            {user && <AccountModal />}
          </nav>
        </div>
      </header>
      <main className="flex-1 flex flex-col min-h-0 mx-auto w-full max-w-5xl px-4 pt-8 pb-4">
        {children}
      </main>
      <footer className="border-t">
        <div className="container mx-auto flex max-w-5xl flex-col gap-3 px-4 py-5 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <p>ezGif</p>
          <nav className="flex flex-wrap gap-x-4 gap-y-2">
            <Link href="/terms" className="hover:text-foreground transition-colors">
              Terms
            </Link>
            <Link href="/privacy" className="hover:text-foreground transition-colors">
              Privacy
            </Link>
            <Link href="/changelog" className="hover:text-foreground transition-colors">
              Changelog
            </Link>
            <Link href="/license" className="hover:text-foreground transition-colors">
              License
            </Link>
          </nav>
        </div>
      </footer>
    </div>
  );
}
