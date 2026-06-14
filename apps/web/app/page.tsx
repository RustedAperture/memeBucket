import { AppShell } from "@/components/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Coffee, GitFork, Search, Sparkles, Image as ImageIcon } from "lucide-react";
import Link from "next/link";

export default function HomePage() {
  return (
    <AppShell>
      <div className="space-y-6">
        <div className="rounded-lg border bg-card text-card-foreground shadow-sm p-8">
          <div className="flex items-center gap-3 mb-4">
            <div className="p-2 bg-muted rounded-md text-primary">
              <Sparkles className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
          </div>
          <p className="text-muted-foreground">
            Welcome to ezGif. Manage your personal Discord media pools.
          </p>
          <div className="mt-6 flex flex-col gap-2 sm:flex-row">
            <Button
              size="lg"
              nativeButton={false}
              className="bg-[#50ACED] hover:scale-105 hover:bg-[#61bcfe]"
              render={
                <Link
                  href="https://ko-fi.com/walnutfox"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Coffee />
                  Support on Ko-fi
                </Link>
              }
            />
            <Button
              size="lg"
              variant="outline"
              nativeButton={false}
              className="hover:scale-105"
              render={
                <Link
                  href="https://github.com/RustedAperture/ezgif"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <GitFork />
                  GitHub
                </Link>
              }
            />
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <Link href="/pools" className="block">
            <Card className="h-full hover:bg-muted/50 transition-colors">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>Pools</CardTitle>
                  <ImageIcon className="w-5 h-5 text-muted-foreground" />
                </div>
                <CardDescription>Organize your media</CardDescription>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  View and manage all your saved media pools.
                </p>
              </CardContent>
            </Card>
          </Link>
          <Link href="/search" className="block">
            <Card className="h-full hover:bg-muted/50 transition-colors">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>Library</CardTitle>
                  <Search className="w-5 h-5 text-muted-foreground" />
                </div>
                <CardDescription>Find saved media</CardDescription>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  Search GIFs and images already saved in your pools.
                </p>
              </CardContent>
            </Card>
          </Link>
        </div>
      </div>
    </AppShell>
  );
}
