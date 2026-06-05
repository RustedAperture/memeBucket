import { AppShell } from "@/components/app-shell";
import { Button } from "@/components/ui/button";

export default function AccountPage() {
  return (
    <AppShell>
      <div className="space-y-4">
        <h1 className="text-2xl font-semibold">Account</h1>
        <div className="flex gap-2">
          <Button variant="outline" render={<a href="/api/account/export" />}>Export data</Button>
          <Button variant="destructive">Delete account</Button>
        </div>
      </div>
    </AppShell>
  );
}
