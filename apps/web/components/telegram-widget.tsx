"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { Button, buttonVariants } from "@/components/ui/button";
import type { VariantProps } from "class-variance-authority";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faTelegram } from "@fortawesome/free-brands-svg-icons";

interface TelegramConfig {
  configured: boolean;
  bot_id?: string;
  username?: string;
}

interface Props extends VariantProps<typeof buttonVariants> {
  authUrl: string;
  label?: string;
  className?: string;
}

export function TelegramWidget({
  authUrl,
  variant = "default",
  size = "default",
  label = "Continue with Telegram",
  className,
}: Props) {
  const [config, setConfig] = useState<TelegramConfig | null>(null);

  useEffect(() => {
    fetch("/api/telegram/config")
      .then(r => r.json())
      .then((data: TelegramConfig) => setConfig(data))
      .catch(() => setConfig({ configured: false }));
  }, []);

  const handleClick = () => {
    if (!config?.configured || !config.bot_id) return;
    const origin = window.location.origin;
    const popupUrl =
      `https://oauth.telegram.org/auth` +
      `?bot_id=${config.bot_id}` +
      `&origin=${encodeURIComponent(origin)}` +
      `&embed=1` +
      `&request_access=write` +
      `&return_to=${encodeURIComponent(authUrl)}`;
    const popup = window.open(popupUrl, "telegram_auth", "width=550,height=470,top=100,left=100");
    if (!popup) return;
    // When the popup closes (after auth + redirect back to our origin), reload so the
    // session cookie takes effect.
    const timer = setInterval(() => {
      if (popup.closed) {
        clearInterval(timer);
        window.location.reload();
      }
    }, 500);
  };

  // Loading: show disabled button to avoid layout shift
  if (config === null) {
    return (
      <Button variant={variant} size={size} className={cn("gap-2", className)} disabled>
        <FontAwesomeIcon icon={faTelegram} className="size-4" />
        {label}
      </Button>
    );
  }

  // Not configured on this server
  if (!config.configured) {
    if (variant === "default") {
      return (
        <p className="text-xs text-muted-foreground text-center">
          Telegram login is not configured on this instance.
        </p>
      );
    }
    // In compact contexts (connected-accounts) just hide it
    return null;
  }

  return (
    <Button
      variant={variant}
      size={size}
      className={cn("gap-2", className)}
      onClick={handleClick}
    >
      <FontAwesomeIcon icon={faTelegram} className="size-4" />
      {label}
    </Button>
  );
}
