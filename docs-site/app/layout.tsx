import type { Metadata, Viewport } from "next";
import "./globals.css";
import { DocsFooter } from "@/components/DocsFooter";
import { Analytics } from "@vercel/analytics/react";

export const metadata: Metadata = {
  metadataBase: new URL('https://docs.horus-registry.dev'),
  title: "HORUS Documentation - Sub-Microsecond IPC for Robotics",
  description: "Official documentation for HORUS robotics framework. Sub-microsecond latency (312-481ns), real-time control, production-ready. 50-500x faster than ROS2.",
  keywords: [
    'robotics framework',
    'robotics operating system',
    'ROS alternative',
    'ROS2 alternative',
    'real-time robotics',
    'IPC framework',
    'distributed robotics',
    'robotics middleware',
    'low latency robotics',
    'real-time control system',
    'embedded robotics',
    'robotics software',
    'AI robotics',
    'physical intelligence',
    'embodied AI',
    'humanoid robot',
    'autonomous robot',
    'robot AI framework',
    'machine learning robotics',
    'computer vision robotics',
    'autonomous systems',
    'intelligent robots',
    'Rust robotics',
    'Python robotics',
    'C++ robotics',
  ],
  icons: {
    icon: [
      { url: '/favicon.ico', sizes: '32x32' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/horus_logo.png', sizes: '192x192', type: 'image/png' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
  },
  openGraph: {
    title: "HORUS Documentation",
    description: "Modern robotics operating system for AI robotics, humanoids, and autonomous systems. Sub-microsecond latency (312-481ns), 50-500x faster than ROS2.",
    url: "https://docs.horus-registry.dev",
    siteName: "HORUS Documentation",
    images: [
      {
        url: '/horus_logo.png',
        width: 192,
        height: 192,
        alt: 'HORUS Logo',
      },
    ],
    locale: 'en_US',
    type: 'website',
  },
  twitter: {
    card: 'summary',
    title: "HORUS Robotics Framework",
    description: "Sub-microsecond framework for AI robotics and humanoids. ROS/ROS2 alternative with 50-500x better performance.",
    images: ['/horus_logo.png'],
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
    description: 'Hybrid Optimized Robotics Unified System. Sub-microsecond IPC (312-481ns), zero-copy shared memory, real-time control framework for AI robotics, humanoids, and autonomous systems. Alternative to ROS and ROS2.',
    softwareVersion: '0.1.3',
    url: 'https://docs.horus-registry.dev',
    keywords: 'robotics framework, robotics operating system, ROS alternative, ROS2 alternative, AI robotics, physical intelligence, embodied AI, humanoid robot, autonomous robot, machine learning robotics, IPC, real-time control, distributed computing, robotics middleware, Rust, Python, C++, low latency, shared memory, pub-sub, sub-microsecond, embedded robotics, real-time system, intelligent robots',
    programmingLanguage: ['Rust', 'Python', 'C++'],
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
