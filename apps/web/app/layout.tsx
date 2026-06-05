import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Random Media Bot",
  description: "Manage personal Discord media categories.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased">{children}</body>
    </html>
  );
}
