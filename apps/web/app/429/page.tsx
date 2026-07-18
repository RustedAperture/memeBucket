"use client";

import Link from "next/link";
import { Home, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StatusPage } from "@/components/status-page";

export default function TooManyRequests() {
  return (
    <StatusPage
      code="429"
      title="Too many requests"
      description="Please wait 1 minute before trying again."
    >
      <Button nativeButton={false} render={<Link href="/" />}>
        <Home />
        Dashboard
      </Button>
      <Button variant="outline" onClick={() => window.location.reload()}>
        <RefreshCw />
        Try again
      </Button>
    </StatusPage>
  );
}
