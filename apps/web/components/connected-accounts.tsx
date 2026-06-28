"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { TelegramWidget } from "@/components/telegram-widget";
import { apiGet } from "@/lib/api";
import { toast } from "sonner";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faDiscord } from "@fortawesome/free-brands-svg-icons";

type Identity = { provider: string; display_name?: string; avatar_url?: string };

function getCsrfToken(): string {
  if (typeof document === "undefined") return "";
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match ? match[1] : "";
}

export function ConnectedAccounts() {
  const [identities, setIdentities] = useState<Identity[]>([]);
  const [telegramLinkUrl, setTelegramLinkUrl] = useState("");

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

  useEffect(() => {
    const apiBase = process.env.NEXT_PUBLIC_API_URL
      ? `${process.env.NEXT_PUBLIC_API_URL}/auth/telegram/callback`
      : "/auth/telegram/callback";
    setTelegramLinkUrl(`${apiBase}?link_token=${getCsrfToken()}`);
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
          <Button
            variant="outline"
            size="sm"
            className="cursor-pointer border-[#5865F2] text-[#5865F2] hover:bg-[#5865F2]/10 hover:text-[#5865F2]"
            render={<a href="/auth/discord/start" />}
          >
            <FontAwesomeIcon icon={faDiscord} className="size-3.5" />
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
        ) : telegramLinkUrl ? (
          <TelegramWidget
            authUrl={telegramLinkUrl}
            variant="outline"
            size="sm"
            label="Connect"
            className="cursor-pointer border-[#2CA5E0] text-[#2CA5E0] hover:bg-[#2CA5E0]/10 hover:text-[#2CA5E0]"
          />
        ) : null}
      </div>
    </section>
  );
}
