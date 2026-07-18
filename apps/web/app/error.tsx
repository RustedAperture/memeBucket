"use client";

import Link from "next/link";
import { Home, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StatusPage } from "@/components/status-page";

export default function Error({ reset }: { reset: () => void }) {
  return (
    <StatusPage
      code="500"
      title="Something went wrong"
      description="We couldn’t load this page. Try again, or return to the dashboard."
    >
      <Button onClick={() => reset()}>
        <RefreshCw />
        Try again
      </Button>
      <Button variant="outline" nativeButton={false} render={<Link href="/" />}>
        <Home />
        Dashboard
      </Button>
    </StatusPage>
  );
}
