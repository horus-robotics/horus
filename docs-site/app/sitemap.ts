import { MetadataRoute } from 'next';

export default function sitemap(): MetadataRoute.Sitemap {
  const baseUrl = 'https://docs.horus.dev';

  // All doc routes from generateStaticParams
  const docRoutes = [
    'goals',
    'roadmap',
    'architecture',
    'benchmarks',
    'getting-started',
    'getting-started/installation',
    'getting-started/quick-start',
    'node-macro',
    'dashboard',
    'parameters',
    'cli-reference',
    'package-management',
    'environment-management',
    'authentication',
    'remote-deployment',
    'library-reference',
    'core',
    'core/link',
    'core/hub',
    'core-concepts-nodes',
    'core-concepts-hub',
    'core-concepts-link',
    'core-concepts-scheduler',
    'core-concepts-shared-memory',
    'message-types',
    'examples',
    'performance',
    'python-bindings',
    'c-bindings',
    'installation',
    'quick-start',
    'guides/robot-controller',
    'guides/sensor-fusion',
    'guides/performance',
    'api',
    'api-node',
    'api-hub',
    'api-link',
    'api-scheduler',
    'ai-integration',
  ];

  const routes: MetadataRoute.Sitemap = [
    {
      url: baseUrl,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: 1,
    },
    ...docRoutes.map((route) => ({
      url: `${baseUrl}/${route}`,
      lastModified: new Date(),
      changeFrequency: 'weekly' as const,
      priority: 0.8,
    })),
  ];

  return routes;
}
