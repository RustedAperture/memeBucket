"use client";

import { useEffect, useState } from "react";
import { AppShell } from "@/components/app-shell";
import { Button } from "@/components/ui/button";
import { Download, Apple, Monitor, Cpu } from "lucide-react";

const REPO = "RustedAperture/memeBucket";

type Platform = "mac-arm" | "mac-intel" | "windows" | "linux-appimage" | "linux-deb" | "unknown";

interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface Release {
  tag_name: string;
  published_at: string;
  assets: Asset[];
}

interface PlatformAsset {
  label: string;
  description: string;
  icon: React.ReactNode;
  asset: Asset | undefined;
}

function detectPlatform(): Platform {
  if (typeof navigator === "undefined") return "unknown";
  const ua = navigator.userAgent;
  if (/Mac/.test(ua)) {
    // Can't reliably detect Apple Silicon from the browser UA, default to ARM
    // since most recent Macs are Apple Silicon.
    return "mac-arm";
  }
  if (/Win/.test(ua)) return "windows";
  if (/Linux/.test(ua)) return "linux-appimage";
  return "unknown";
}

function categorizeAssets(assets: Asset[]): Record<Platform, Asset | undefined> {
  return {
    "mac-arm": assets.find((a) => a.name.endsWith(".dmg") && a.name.includes("aarch64")),
    "mac-intel": assets.find((a) => a.name.endsWith(".dmg") && (a.name.includes("x64") || a.name.includes("x86_64"))),
    "windows": assets.find((a) => a.name.endsWith(".msi")),
    "linux-appimage": assets.find((a) => a.name.endsWith(".AppImage")),
    "linux-deb": assets.find((a) => a.name.endsWith(".deb")),
    "unknown": undefined,
  };
}

function formatSize(bytes: number) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
}

export default function DownloadPage() {
  const [release, setRelease] = useState<Release | null>(null);
  const [error, setError] = useState(false);
  const [platform, setPlatform] = useState<Platform>("unknown");
  const [origin, setOrigin] = useState("");

  useEffect(() => {
    setPlatform(detectPlatform());
    setOrigin(window.location.origin);
    fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
      .then((r) => r.json())
      .then((data) => {
        if (data.tag_name) setRelease(data);
        else setError(true);
      })
      .catch(() => setError(true));
  }, []);

  const categorized = release ? categorizeAssets(release.assets) : null;

  const platforms: { key: Platform; label: string; description: string; icon: React.ReactNode }[] = [
    { key: "mac-arm", label: "macOS (Apple Silicon)", description: "M1 and later", icon: <Apple className="w-5 h-5" /> },
    { key: "mac-intel", label: "macOS (Intel)", description: "x86_64", icon: <Apple className="w-5 h-5" /> },
    { key: "windows", label: "Windows", description: ".msi installer", icon: <Monitor className="w-5 h-5" /> },
    { key: "linux-appimage", label: "Linux", description: "AppImage", icon: <Cpu className="w-5 h-5" /> },
    { key: "linux-deb", label: "Linux", description: ".deb package", icon: <Cpu className="w-5 h-5" /> },
  ];

  const primaryPlatform = platforms.find((p) => p.key === platform) ?? platforms[0];
  const otherPlatforms = platforms.filter((p) => p.key !== primaryPlatform.key);

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-10 py-8">
        <div className="space-y-2">
          <h1 className="text-3xl font-bold tracking-tight">Download</h1>
          <p className="text-muted-foreground">
            The memeBucket Picker is a lightweight desktop overlay for quickly copying media into Discord.
          </p>
          {release && (
            <p className="text-sm text-muted-foreground">
              {release.tag_name} &middot; Released {formatDate(release.published_at)}
            </p>
          )}
        </div>

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
            Could not fetch the latest release. Check the{" "}
            <a
              href={`https://github.com/${REPO}/releases`}
              target="_blank"
              rel="noopener noreferrer"
              className="underline"
            >
              GitHub releases page
            </a>{" "}
            directly.
          </div>
        )}

        {/* Primary download */}
        {!error && (
          <div className="rounded-lg border bg-card p-6 space-y-4">
            <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground">
              Recommended for your device
            </p>
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-muted rounded-md text-foreground">
                  {primaryPlatform.icon}
                </div>
                <div>
                  <p className="font-semibold">{primaryPlatform.label}</p>
                  <p className="text-sm text-muted-foreground">{primaryPlatform.description}</p>
                </div>
              </div>
              {categorized ? (
                categorized[primaryPlatform.key] ? (
                  <Button size="lg" render={<a href={categorized[primaryPlatform.key]!.browser_download_url} download />}>
                    <Download className="w-4 h-4" />
                    Download
                    <span className="text-xs opacity-70 ml-1">
                      {formatSize(categorized[primaryPlatform.key]!.size)}
                    </span>
                  </Button>
                ) : (
                  <Button size="lg" disabled>
                    Not available
                  </Button>
                )
              ) : (
                <Button size="lg" disabled>
                  Loading…
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Other platforms */}
        {!error && (
          <div className="space-y-3">
            <p className="text-sm font-medium text-muted-foreground">Other platforms</p>
            <div className="divide-y rounded-lg border bg-card">
              {otherPlatforms.map((p) => {
                const asset = categorized?.[p.key];
                return (
                  <div key={p.key} className="flex items-center justify-between px-4 py-3 gap-4">
                    <div className="flex items-center gap-3">
                      <span className="text-muted-foreground">{p.icon}</span>
                      <div>
                        <p className="text-sm font-medium">{p.label}</p>
                        <p className="text-xs text-muted-foreground">{p.description}</p>
                      </div>
                    </div>
                    {asset ? (
                      <Button variant="outline" size="sm" render={<a href={asset.browser_download_url} download />}>
                        <Download className="w-3.5 h-3.5" />
                        {formatSize(asset.size)}
                      </Button>
                    ) : (
                      <Button variant="outline" size="sm" disabled>
                        {categorized ? "N/A" : "…"}
                      </Button>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <p className="text-xs text-muted-foreground">
          macOS users: right-click → Open on first launch if you see a security warning (app is not notarized).
        </p>

        {/* Getting started */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold tracking-tight">Getting started</h2>
          <ol className="space-y-6">
            {[
              {
                step: "1",
                title: "Install and launch",
                body: "After installing, the app won't open a regular window — it lives in your menu bar (macOS) or system tray (Windows / Linux). Look for the memeBucket icon there.",
              },
              {
                step: "2",
                title: "Connect to your server",
                body: (
                  <>
                    Click the tray icon to open the picker, then click the settings icon (⚙) and enter your memeBucket server URL — for example{" "}
                    <kbd className="rounded border bg-muted px-1.5 py-0.5 text-xs font-mono">{origin || "https://your-server.com"}</kbd>
                    . Hit Save.
                  </>
                ),
              },
              {
                step: "3",
                title: "Open the picker anywhere",
                body: (
                  <>
                    Press{" "}
                    <kbd className="rounded border bg-muted px-1.5 py-0.5 text-xs font-mono">⌘ Shift M</kbd>
                    {" "}(macOS) or{" "}
                    <kbd className="rounded border bg-muted px-1.5 py-0.5 text-xs font-mono">Ctrl Shift M</kbd>
                    {" "}(Windows / Linux) from any app to toggle the picker.
                  </>
                ),
              },
              {
                step: "4",
                title: "Search and paste",
                body: "Type to search your buckets. Use arrow keys to navigate, Enter to paste the selected image directly into the active app. Escape closes the picker.",
              },
            ].map(({ step, title, body }) => (
              <li key={step} className="flex gap-4">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground text-sm font-semibold">
                  {step}
                </span>
                <div className="space-y-1 pt-0.5">
                  <p className="font-medium">{title}</p>
                  <p className="text-sm text-muted-foreground">{body}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </AppShell>
  );
}
