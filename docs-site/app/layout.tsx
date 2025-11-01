import type { Metadata, Viewport } from "next";
import "./globals.css";
import { DocsFooter } from "@/components/DocsFooter";
import { Analytics } from "@vercel/analytics/react";

export const metadata: Metadata = {
  title: "HORUS Documentation - Ultra-Low Latency IPC for Robotics",
  description: "Official documentation for HORUS distributed computing framework. 29ns latency, real-time control, production-ready.",
  icons: {
    icon: [
      { url: '/favicon.ico', sizes: '32x32' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
  },
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
  const jsonLd = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'HORUS',
    applicationCategory: 'DeveloperApplication',
    operatingSystem: 'Linux, macOS, Windows',
    description: 'Ultra-low latency IPC framework for robotics and real-time control systems. 29ns message passing, zero-copy shared memory, production-ready.',
    softwareVersion: '0.1.0',
    url: 'https://docs.horus.dev',
    keywords: 'robotics framework, IPC, real-time control, distributed computing, Rust, low latency, shared memory, pub-sub',
    programmingLanguage: ['Rust', 'Python', 'C'],
    creator: {
      '@type': 'Organization',
      name: 'HORUS Team',
    },
    offers: {
      '@type': 'Offer',
      price: '0',
      priceCurrency: 'USD',
    },
  };

  return (
    <html lang="en">
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </head>
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
