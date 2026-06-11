"use client";

import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useState } from "react";

import { cn } from "@/lib/utils";

const OPTIONS = [
  { value: "light", label: "Light mode", icon: Sun },
  { value: "dark", label: "Dark mode", icon: Moon },
  { value: "system", label: "Auto mode", icon: Monitor },
] as const;

type ThemeValue = (typeof OPTIONS)[number]["value"];

function storedTheme(fallback: string): ThemeValue {
  if (typeof window === "undefined") {
    return fallback === "light" || fallback === "dark" ? fallback : "system";
  }

  const stored = window.localStorage.getItem("theme");
  if (stored === "light" || stored === "dark" || stored === "system") {
    return stored;
  }

  return fallback === "light" || fallback === "dark" ? fallback : "system";
}

export function ThemeToggle() {
  const { theme = "system", setTheme } = useTheme();
  const [selectedTheme, setSelectedTheme] = useState<ThemeValue>(() => storedTheme(theme));

  return (
    <div
      className="flex h-8 items-center rounded-4xl border bg-background p-0.5"
      aria-label="Theme"
    >
      {OPTIONS.map(({ value, label, icon: Icon }) => {
        const active = selectedTheme === value;

        return (
          <button
            key={value}
            type="button"
            title={label}
            aria-label={label}
            aria-pressed={active}
            className={cn(
              "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-4xl text-muted-foreground outline-none transition-colors focus-visible:ring-3 focus-visible:ring-ring/30",
              active
                ? "bg-primary text-primary-foreground"
                : "hover:bg-transparent hover:text-foreground"
            )}
            onClick={() => {
              setSelectedTheme(value);
              setTheme(value);
            }}
          >
            <Icon className="size-3" />
          </button>
        );
      })}
    </div>
  );
}
