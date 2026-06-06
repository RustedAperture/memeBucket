import type { Metadata } from "next";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";
import { UserProvider } from "@/components/user-provider";
import { UsernameModal } from "@/components/username-modal";
import "./globals.css";

export const metadata: Metadata = {
  title: "ezGif",
  description: "Personal Discord media categories.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="font-sans">
      <body className="antialiased min-h-screen bg-background selection:bg-primary/30">
        <UserProvider>
          <TooltipProvider>{children}</TooltipProvider>
          <UsernameModal />
        </UserProvider>
        <Toaster position="bottom-center" />
      </body>
    </html>
  );
}
