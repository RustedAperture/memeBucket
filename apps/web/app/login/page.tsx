import { Button } from "@/components/ui/button";

export default function LoginPage() {
  return (
    <main className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm space-y-4">
        <h1 className="text-2xl font-semibold">Sign in</h1>
        <p className="text-sm text-muted-foreground">Use Discord to manage your media categories.</p>
        <Button className="w-full" render={<a href="/auth/discord/start" />}>Continue with Discord</Button>
      </div>
    </main>
  );
}
