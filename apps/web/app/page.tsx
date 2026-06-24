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
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8 items-center">
            <div className="md:col-span-2 space-y-4">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-muted rounded-md text-primary">
                  <Sparkles className="w-6 h-6" />
                </div>
                <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
              </div>
              <p className="text-muted-foreground">
                Welcome to memeBucket. Manage your personal Discord media buckets.
              </p>
              <div className="flex flex-col gap-2 sm:flex-row pt-2">
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
                      href="https://github.com/RustedAperture/memeBucket"
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
            <div className="relative aspect-video md:aspect-square w-full max-w-[280px] mx-auto overflow-hidden rounded-lg border shadow-sm">
              <img
                src="https://i.kym-cdn.com/entries/icons/original/000/038/583/bucket.jpg"
                alt="This is a bucket"
                className="object-cover w-full h-full"
              />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <Link href="/buckets" className="block">
            <Card className="h-full hover:bg-muted/50 transition-colors">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>Buckets</CardTitle>
                  <ImageIcon className="w-5 h-5 text-muted-foreground" />
                </div>
                <CardDescription>Organize your media</CardDescription>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  View and manage all your saved media buckets.
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
                  Search GIFs and images already saved in your buckets.
                </p>
              </CardContent>
            </Card>
          </Link>
        </div>
      </div>
    </AppShell>
  );
}
