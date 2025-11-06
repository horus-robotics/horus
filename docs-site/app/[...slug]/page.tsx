import { getDoc } from '@/lib/mdx';
import { DocsLayout } from '@/components/DocsLayout';
import { TableOfContents } from '@/components/TableOfContents';
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
        <article className="prose prose-invert max-w-none prose-headings:scroll-mt-20 prose-p:text-[var(--text-secondary)] prose-p:leading-relaxed prose-li:text-[var(--text-secondary)]">
          {doc.content}
        </article>
      </main>
      <TableOfContents />
    </DocsLayout>
  );
}

export async function generateStaticParams() {
  // Define all doc routes
  return [
    { slug: ['goals'] },
    { slug: ['roadmap'] },
    { slug: ['architecture'] },
    { slug: ['benchmarks'] },
    { slug: ['getting-started'] },
    { slug: ['getting-started', 'installation'] },
    { slug: ['getting-started', 'quick-start'] },
    { slug: ['node-macro'] },
    { slug: ['message-macro'] },
    { slug: ['dashboard'] },
    { slug: ['parameters'] },
    { slug: ['cli-reference'] },
    { slug: ['package-management'] },
    { slug: ['environment-management'] },
    { slug: ['authentication'] },
    { slug: ['remote-deployment'] },
    { slug: ['library-reference'] },
    { slug: ['using-prebuilt-nodes'] },
    { slug: ['core'] },
    { slug: ['core', 'link'] },
    { slug: ['core', 'hub'] },
    { slug: ['core-concepts-nodes'] },
    { slug: ['core-concepts-hub'] },
    { slug: ['core-concepts-link'] },
    { slug: ['core-concepts-scheduler'] },
    { slug: ['core-concepts-shared-memory'] },
    { slug: ['message-types'] },
    { slug: ['examples'] },
    { slug: ['performance'] },
    { slug: ['python-bindings'] },
    { slug: ['c-bindings'] },
    { slug: ['multi-language'] },
    { slug: ['installation'] },
    { slug: ['quick-start'] },
    { slug: ['guides', 'robot-controller'] },
    { slug: ['guides', 'sensor-fusion'] },
    { slug: ['guides', 'performance'] },
    { slug: ['api'] },
    { slug: ['api-node'] },
    { slug: ['api-hub'] },
    { slug: ['api-link'] },
    { slug: ['api-scheduler'] },
    { slug: ['ai-integration'] },
    { slug: ['simulation'] },
    { slug: ['troubleshooting'] },
  ];
}
