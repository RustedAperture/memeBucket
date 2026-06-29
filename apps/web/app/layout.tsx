import type { Metadata, Viewport } from "next";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "@/components/theme-provider";
import { UserProvider } from "@/components/user-provider";
import { UsernameModal } from "@/components/username-modal";
import "./globals.css";

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
};

export const metadata: Metadata = {
  title: "memeBucket",
  description: "Personal Discord media categories.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="font-sans" suppressHydrationWarning>
      <body className="antialiased min-h-screen bg-background selection:bg-primary/30">
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
          <UserProvider>
            <TooltipProvider>{children}</TooltipProvider>
            <UsernameModal />
          </UserProvider>
          <Toaster position="bottom-center" />
        </ThemeProvider>
      </body>
    </html>
  );
}
