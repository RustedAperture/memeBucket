"use client";

import { useEffect, useState } from "react";
import Script from "next/script";
import { Button } from "@/components/ui/button";
import { apiGet } from "@/lib/api";
import { toast } from "sonner";

type Identity = { provider: string; display_name?: string; avatar_url?: string };

const TELEGRAM_BOT_USERNAME = process.env.NEXT_PUBLIC_TELEGRAM_BOT_USERNAME ?? "";

function getCsrfToken(): string {
  if (typeof document === "undefined") return "";
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match ? match[1] : "";
}

export function ConnectedAccounts() {
  const [identities, setIdentities] = useState<Identity[]>([]);

  const fetchIdentities = async () => {
    try {
      const data = await apiGet<Identity[]>("/api/account/identities");
      setIdentities(data);
    } catch {
      // silently ignore errors on initial load
    }
  };

  useEffect(() => {
    fetchIdentities();
  }, []);

  const isLinked = (provider: string) => identities.some(i => i.provider === provider);

  const disconnect = async (provider: string) => {
    try {
      const res = await fetch(`/api/account/identities/${provider}`, {
        method: "DELETE",
        credentials: "include",
        headers: { "X-CSRF-Token": getCsrfToken() },
      });
      if (!res.ok) {
        throw new Error((await res.text()) || "Failed to disconnect");
      }
      await fetchIdentities();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to disconnect account");
    }
  };

  return (
    <section id="connected-accounts" className="space-y-3">
      <h3 className="text-sm font-medium text-muted-foreground">Connected accounts</h3>

      <div className="flex items-center justify-between py-2 border-b">
        <div>
          <p className="text-sm font-medium">Discord</p>
          {isLinked("discord") && (
            <p className="text-xs text-muted-foreground">
              {identities.find(i => i.provider === "discord")?.display_name ?? "Unknown"}
            </p>
          )}
        </div>
        {isLinked("discord") ? (
          <Button
            variant="outline"
            size="sm"
            onClick={() => disconnect("discord")}
            disabled={identities.length <= 1}
          >
            Disconnect
          </Button>
        ) : (
          <Button variant="outline" size="sm" render={<a href="/auth/discord/start" />}>
            Connect
          </Button>
        )}
      </div>

      <div className="flex items-center justify-between py-2">
        <div>
          <p className="text-sm font-medium">Telegram</p>
          {isLinked("telegram") && (
            <p className="text-xs text-muted-foreground">
              {identities.find(i => i.provider === "telegram")?.display_name ?? "Unknown"}
            </p>
          )}
        </div>
        {isLinked("telegram") ? (
          <Button
            variant="outline"
            size="sm"
            onClick={() => disconnect("telegram")}
            disabled={identities.length <= 1}
          >
            Disconnect
          </Button>
        ) : TELEGRAM_BOT_USERNAME ? (
          <Script
            src="https://telegram.org/js/telegram-widget.js?22"
            strategy="afterInteractive"
            data-telegram-login={TELEGRAM_BOT_USERNAME}
            data-size="small"
            data-auth-url="/auth/telegram/callback"
            data-request-access="write"
          />
        ) : null}
      </div>
    </section>
  );
}
