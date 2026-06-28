import Script from "next/script";
import { Button } from "@/components/ui/button";

const TELEGRAM_BOT_USERNAME = process.env.NEXT_PUBLIC_TELEGRAM_BOT_USERNAME ?? "";
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

        <Button className="w-full" render={<a href="/auth/discord/start" />}>
          Continue with Discord
        </Button>

        {TELEGRAM_BOT_USERNAME && (
          <div className="flex justify-center">
            <Script
              src="https://telegram.org/js/telegram-widget.js?22"
              strategy="afterInteractive"
              data-telegram-login={TELEGRAM_BOT_USERNAME}
              data-size="large"
              data-radius="8"
              data-auth-url={TELEGRAM_AUTH_URL}
              data-request-access="write"
            />
          </div>
        )}

        {!TELEGRAM_BOT_USERNAME && (
          <p className="text-xs text-muted-foreground text-center">
            Telegram login is not configured on this instance.
          </p>
        )}
      </div>
    </main>
  );
}
