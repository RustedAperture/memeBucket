"use client";

import { useEffect, useState } from "react";
import { AppShell } from "@/components/app-shell";
import { Button } from "@/components/ui/button";
import { Download, Apple, Monitor, Cpu, Smartphone } from "lucide-react";

const REPO = "RustedAperture/memeBucket";

type Platform = "mac-arm" | "windows" | "linux-appimage" | "linux-deb" | "ios" | "android" | "unknown";

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
  if (/iPhone|iPad|iPod/.test(ua)) return "ios";
  if (/Android/.test(ua)) return "android";
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
    "windows": assets.find((a) => a.name.endsWith(".msi")),
    "linux-appimage": assets.find((a) => a.name.endsWith(".AppImage")),
    "linux-deb": assets.find((a) => a.name.endsWith(".deb")),
    "ios": undefined,
    "android": undefined,
    "unknown": undefined,
  };
}

function isMobilePlatform(p: Platform) {
  return p === "ios" || p === "android";
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

        {/* Mobile primary card */}
        {isMobilePlatform(platform) && (
          <div className="rounded-lg border bg-card p-6 space-y-4">
            <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground">
              Recommended for your device
            </p>
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-muted rounded-md text-foreground">
                <Smartphone className="w-5 h-5" />
              </div>
              <div>
                <p className="font-semibold">{platform === "ios" ? "iPhone / iPad" : "Android"}</p>
                <p className="text-sm text-muted-foreground">Add to Home Screen</p>
              </div>
            </div>
            {platform === "ios" ? (
              <ol className="space-y-2 text-sm text-muted-foreground list-decimal list-inside">
                <li>Open <span className="font-medium text-foreground">{origin ? `${origin}/picker` : "memeBucket/picker"}</span> in Safari</li>
                <li>Tap the <span className="font-medium text-foreground">Share</span> button (the box with an arrow pointing up)</li>
                <li>Scroll down and tap <span className="font-medium text-foreground">Add to Home Screen</span></li>
                <li>Tap <span className="font-medium text-foreground">Add</span> — the picker opens like an app</li>
              </ol>
            ) : (
              <ol className="space-y-2 text-sm text-muted-foreground list-decimal list-inside">
                <li>Open <span className="font-medium text-foreground">{origin ? `${origin}/picker` : "memeBucket/picker"}</span> in Chrome</li>
                <li>Tap the <span className="font-medium text-foreground">menu (⋮)</span> in the top-right corner</li>
                <li>Tap <span className="font-medium text-foreground">Add to Home Screen</span> or <span className="font-medium text-foreground">Install app</span></li>
                <li>Tap <span className="font-medium text-foreground">Add</span> — the picker opens like an app</li>
              </ol>
            )}
          </div>
        )}

        {/* Primary download */}
        {!error && !isMobilePlatform(platform) && (
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
        {!error && !isMobilePlatform(platform) && (
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

        {/* Mobile section for desktop visitors */}
        {!isMobilePlatform(platform) && (
          <div className="rounded-lg border bg-card p-6 space-y-3">
            <div className="flex items-center gap-2">
              <Smartphone className="w-4 h-4 text-muted-foreground" />
              <p className="text-sm font-medium">iPhone, iPad &amp; Android</p>
            </div>
            <p className="text-sm text-muted-foreground">
              On mobile, add the picker to your home screen for quick access — no install required.
            </p>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-md bg-muted p-3 space-y-1.5">
                <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">iOS (Safari)</p>
                <ol className="space-y-1 text-xs text-muted-foreground list-decimal list-inside">
                  <li>Open <span className="font-medium text-foreground">{origin ? `${origin}/picker` : "memeBucket/picker"}</span></li>
                  <li>Tap <span className="font-medium text-foreground">Share →  Add to Home Screen</span></li>
                  <li>Tap <span className="font-medium text-foreground">Add</span></li>
                </ol>
              </div>
              <div className="rounded-md bg-muted p-3 space-y-1.5">
                <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Android (Chrome)</p>
                <ol className="space-y-1 text-xs text-muted-foreground list-decimal list-inside">
                  <li>Open <span className="font-medium text-foreground">{origin ? `${origin}/picker` : "memeBucket/picker"}</span></li>
                  <li>Tap <span className="font-medium text-foreground">Menu (⋮) → Add to Home Screen</span></li>
                  <li>Tap <span className="font-medium text-foreground">Add</span></li>
                </ol>
              </div>
            </div>
          </div>
        )}

        {!isMobilePlatform(platform) && (
        <div className="rounded-lg border bg-card p-4 space-y-2">
          <p className="text-sm font-medium">macOS: "app is damaged" warning</p>
          <p className="text-sm text-muted-foreground">
            macOS quarantines unsigned apps downloaded from the internet. If you see a "damaged" or "can't be opened" error, run this in Terminal:
          </p>
          <pre className="rounded-md bg-muted px-3 py-2 text-xs font-mono">xattr -cr "/Applications/memeBucket Picker.app"</pre>
          <p className="text-xs text-muted-foreground">Then open the app normally.</p>
        </div>
        )}

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
