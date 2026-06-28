import { Button } from "@/components/ui/button";
import { TelegramWidget } from "@/components/telegram-widget";

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
            <TelegramWidget botUsername={TELEGRAM_BOT_USERNAME} authUrl={TELEGRAM_AUTH_URL} />
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
