import { getDoc } from '@/lib/mdx';
import { DocsSidebar } from '@/components/DocsSidebar';
import { TableOfContents } from '@/components/TableOfContents';
import { notFound } from 'next/navigation';

interface PageProps {
  params: {
    slug: string[];
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
    <div className="flex">
      <DocsSidebar />
      <main className="flex-1 max-w-4xl mx-auto px-8 py-12">
        <article className="prose prose-invert max-w-none">
          {doc.content}
        </article>
      </main>
      <TableOfContents />
    </div>
  );
}

export async function generateStaticParams() {
  // Define all doc routes
  return [
    { slug: ['goals'] },
    { slug: ['architecture'] },
    { slug: ['benchmarks'] },
    { slug: ['getting-started'] },
    { slug: ['getting-started', 'installation'] },
    { slug: ['getting-started', 'quick-start'] },
    { slug: ['node-macro'] },
    { slug: ['dashboard'] },
    { slug: ['parameters'] },
    { slug: ['cli-reference'] },
    { slug: ['package-management'] },
    { slug: ['environment-management'] },
    { slug: ['marketplace'] },
    { slug: ['authentication'] },
    { slug: ['remote-deployment'] },
    { slug: ['library-reference'] },
    { slug: ['core'] },
    { slug: ['core', 'link'] },
    { slug: ['core', 'hub'] },
    { slug: ['core', 'iceoryx2'] },
    { slug: ['core-concepts-nodes'] },
    { slug: ['core-concepts-hub'] },
    { slug: ['core-concepts-scheduler'] },
    { slug: ['core-concepts-shared-memory'] },
    { slug: ['message-types'] },
    { slug: ['examples'] },
    { slug: ['performance'] },
    { slug: ['python-bindings'] },
    { slug: ['c-bindings'] },
    { slug: ['installation'] },
    { slug: ['quick-start'] },
    { slug: ['guides', 'robot-controller'] },
    { slug: ['guides', 'sensor-fusion'] },
    { slug: ['guides', 'performance'] },
    { slug: ['api'] },
    { slug: ['api-node'] },
    { slug: ['api-hub'] },
    { slug: ['api-scheduler'] },
  ];
}
