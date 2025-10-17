import type { Metadata } from "next";
import "./globals.css";
import { DocsNav } from "@/components/DocsNav";
import { DocsFooter } from "@/components/DocsFooter";

export const metadata: Metadata = {
  title: "HORUS Documentation - Ultra-Low Latency IPC for Robotics",
  description: "Official documentation for HORUS distributed computing framework. 29ns latency, real-time control, production-ready.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="font-mono">
        <DocsNav />
        <main className="min-h-screen">
          {children}
        </main>
        <DocsFooter />
      </body>
    </html>
  );
}
