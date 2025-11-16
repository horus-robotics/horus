import { getDoc } from '@/lib/mdx';
import { DocsLayout } from '@/components/DocsLayout';
import { TableOfContents } from '@/components/TableOfContents';
import { Breadcrumb } from '@/components/Breadcrumb';
import { notFound } from 'next/navigation';
import type { Metadata } from 'next';

interface PageProps {
  params: {
    slug: string[];
  };
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = params;
  const docPath = ['docs', ...slug];
  const doc = await getDoc(docPath);

  if (!doc) {
    return {
      title: 'Page Not Found',
      description: 'The requested page could not be found.',
    };
  }

  const title = doc.frontmatter.title || 'HORUS Documentation';
  const description = doc.frontmatter.description || 'Official documentation for HORUS distributed computing framework. 29ns latency, real-time control, production-ready.';
  const url = `https://docs.horus.dev/${slug.join('/')}`;

  return {
    title: `${title} | HORUS Docs`,
    description,
    keywords: [
      'HORUS',
      'robotics framework',
      'low latency IPC',
      'real-time control',
      'distributed computing',
      'Rust robotics',
      'shared memory',
      'pub-sub messaging',
      'robot middleware',
      'ROS alternative',
    ],
    authors: [{ name: 'HORUS Team' }],
    openGraph: {
      title,
      description,
      url,
      siteName: 'HORUS Documentation',
      type: 'article',
      images: [
        {
          url: 'https://docs.horus.dev/og-image.png',
          width: 1200,
          height: 630,
          alt: 'HORUS - Ultra-Low Latency IPC for Robotics',
        },
      ],
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description,
      images: ['https://docs.horus.dev/og-image.png'],
    },
    alternates: {
      canonical: url,
    },
  };
}

export default async function DocPage({ params }: PageProps) {
  const { slug } = params;

  // Always prepend 'docs' to the path
  const docPath = ['docs', ...slug];

  const doc = await getDoc(docPath);

  if (!doc) {
    notFound();
  }

  return (
    <DocsLayout>
      <main className="flex-1 w-full max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 sm:py-12">
        <Breadcrumb />
        <article className="prose prose-invert max-w-none prose-headings:scroll-mt-20 prose-p:text-[var(--text-secondary)] prose-p:leading-relaxed prose-li:text-[var(--text-secondary)]">
          {doc.content}
        </article>
      </main>
      <TableOfContents />
    </DocsLayout>
  );
}

export async function generateStaticParams() {
  const fs = require('fs');
  const path = require('path');

  const contentDir = path.join(process.cwd(), 'content/docs');
  const routes: { slug: string[] }[] = [];

  // Recursively find all .mdx files
  function findMdxFiles(dir: string, basePath: string[] = []): void {
    const files = fs.readdirSync(dir);

    for (const file of files) {
      const filePath = path.join(dir, file);
      const stat = fs.statSync(filePath);

      if (stat.isDirectory()) {
        // Recurse into subdirectory
        findMdxFiles(filePath, [...basePath, file]);
      } else if (file.endsWith('.mdx')) {
        // Add route for this MDX file
        const fileName = file.replace(/\.mdx$/, '');

        // For index.mdx files, use the directory path without 'index'
        if (fileName === 'index') {
          // Only add if basePath is not empty (we don't want a route for root index.mdx)
          if (basePath.length > 0) {
            routes.push({ slug: basePath });
          }
        } else {
          routes.push({ slug: [...basePath, fileName] });
        }
      }
    }
  }

  findMdxFiles(contentDir);

  return routes;
}
