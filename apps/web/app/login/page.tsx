"use client";

import { Button } from "@/components/ui/button";
import { TelegramWidget } from "@/components/telegram-widget";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faDiscord } from "@fortawesome/free-brands-svg-icons";

const TELEGRAM_AUTH_URL = process.env.NEXT_PUBLIC_API_URL
  ? `${process.env.NEXT_PUBLIC_API_URL}/auth/telegram/callback`
  : "/auth/telegram/callback";

export default function LoginPage() {
  return (
    <main className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm space-y-4">
        <h1 className="text-2xl font-semibold">Sign in</h1>
        <p className="text-sm text-muted-foreground">
          Connect your account to manage your media buckets.
        </p>

        <Button
          className="w-full gap-2 cursor-pointer bg-[#5865F2] hover:bg-[#5865F2]/80 text-white"
          render={<a href="/auth/discord/start" />}
        >
          <FontAwesomeIcon icon={faDiscord} className="size-4" />
          Continue with Discord
        </Button>

        <TelegramWidget
          authUrl={TELEGRAM_AUTH_URL}
          className="w-full cursor-pointer bg-[#2CA5E0] hover:bg-[#2CA5E0]/80 text-white"
        />
      </div>
    </main>
  );
}
