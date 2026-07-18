import Link from "next/link";
import { Home, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StatusPage } from "@/components/status-page";

export default function NotFound() {
  return (
    <StatusPage
      code="404"
      title="Page not found"
      description="The page you’re looking for doesn’t exist or may have moved."
    >
      <Button nativeButton={false} render={<Link href="/" />}>
        <Home />
        Dashboard
      </Button>
      <Button variant="outline" nativeButton={false} render={<Link href="/search" />}>
        <Search />
        Browse library
      </Button>
    </StatusPage>
  );
}
