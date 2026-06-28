"use client";

import { useEffect, useRef } from "react";

interface Props {
  botUsername: string;
  authUrl: string;
  size?: "large" | "medium" | "small";
}

export function TelegramWidget({ botUsername, authUrl, size = "large" }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const script = document.createElement("script");
    script.src = "https://telegram.org/js/telegram-widget.js?22";
    script.async = true;
    script.setAttribute("data-telegram-login", botUsername);
    script.setAttribute("data-size", size);
    script.setAttribute("data-radius", "8");
    script.setAttribute("data-auth-url", authUrl);
    script.setAttribute("data-request-access", "write");
    container.appendChild(script);
    return () => {
      container.innerHTML = "";
    };
  }, [botUsername, authUrl, size]);

  return <div ref={containerRef} />;
}
