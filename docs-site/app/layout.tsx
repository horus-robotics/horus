import type { Metadata, Viewport } from "next";
import "./globals.css";
import { DocsFooter } from "@/components/DocsFooter";
import { Analytics } from "@vercel/analytics/react";

export const metadata: Metadata = {
  title: "HORUS Documentation - Ultra-Low Latency IPC for Robotics",
  description: "Official documentation for HORUS distributed computing framework. 29ns latency, real-time control, production-ready.",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 5,
  userScalable: true,
  themeColor: "#16181c",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="font-mono antialiased">
        <main className="min-h-screen">
          {children}
        </main>
        <DocsFooter />
        <Analytics />
      </body>
    </html>
  );
}
