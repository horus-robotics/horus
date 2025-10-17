import fs from 'fs';
import path from 'path';
import matter from 'gray-matter';
import { compileMDX } from 'next-mdx-remote/rsc';
import { codeToHtml } from 'shiki';
import remarkGfm from 'remark-gfm';

const contentDirectory = path.join(process.cwd(), 'content');

export interface DocFrontmatter {
  title: string;
  description?: string;
  section?: string;
  order?: number;
}

export interface DocContent {
  slug: string;
  frontmatter: DocFrontmatter;
  content: React.ReactElement;
}

/**
 * Get all MDX files from a directory
 */
export async function getDocSlugs(dir: string): Promise<string[]> {
  const fullPath = path.join(contentDirectory, dir);

  if (!fs.existsSync(fullPath)) {
    return [];
  }

  const files = fs.readdirSync(fullPath);
  return files
    .filter(file => file.endsWith('.mdx'))
    .map(file => file.replace(/\.mdx$/, ''));
}

/**
 * Get a single MDX document by slug
 */
export async function getDoc(slug: string[]): Promise<DocContent | null> {
  try {
    const filePath = path.join(contentDirectory, ...slug) + '.mdx';

    if (!fs.existsSync(filePath)) {
      return null;
    }

    const source = fs.readFileSync(filePath, 'utf-8');
    const { data, content: mdxContent } = matter(source);

    const { content } = await compileMDX<DocFrontmatter>({
      source: mdxContent,
      options: {
        parseFrontmatter: true,
        mdxOptions: {
          remarkPlugins: [remarkGfm],
          rehypePlugins: [],
        },
      },
      components: {
        h2: ({ children, ...props }: any) => {
          const id = typeof children === 'string'
            ? children.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '')
            : '';
          return (
            <h2 id={id} {...props}>
              {children}
            </h2>
          );
        },
        h3: ({ children, ...props }: any) => {
          const id = typeof children === 'string'
            ? children.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '')
            : '';
          return (
            <h3 id={id} {...props}>
              {children}
            </h3>
          );
        },
        code: ({ children, ...props }: any) => {
          return (
            <code
              className="px-1.5 py-0.5 rounded bg-[var(--surface)] text-[var(--accent)] text-sm font-mono"
              {...props}
            >
              {children}
            </code>
          );
        },
        table: ({ children, ...props }: any) => (
          <div className="overflow-x-auto my-6">
            <table className="min-w-full border-collapse border border-gray-700" {...props}>
              {children}
            </table>
          </div>
        ),
        thead: ({ children, ...props }: any) => (
          <thead className="bg-[var(--surface)]" {...props}>
            {children}
          </thead>
        ),
        tbody: ({ children, ...props }: any) => (
          <tbody {...props}>
            {children}
          </tbody>
        ),
        tr: ({ children, ...props }: any) => (
          <tr className="border-b border-gray-700" {...props}>
            {children}
          </tr>
        ),
        th: ({ children, ...props }: any) => (
          <th className="px-4 py-2 text-left font-semibold text-[var(--accent)]" {...props}>
            {children}
          </th>
        ),
        td: ({ children, ...props }: any) => (
          <td className="px-4 py-2 text-gray-300" {...props}>
            {children}
          </td>
        ),
      },
    });

    return {
      slug: slug.join('/'),
      frontmatter: data as DocFrontmatter,
      content,
    };
  } catch (error) {
    console.error('Error loading doc:', error);
    return null;
  }
}

/**
 * Get all documents in a section with their metadata
 */
export async function getDocsList(section: string): Promise<Array<{ slug: string; frontmatter: DocFrontmatter }>> {
  const slugs = await getDocSlugs(section);
  const docs = await Promise.all(
    slugs.map(async (slug) => {
      const doc = await getDoc([section, slug]);
      return doc ? { slug, frontmatter: doc.frontmatter } : null;
    })
  );

  return docs
    .filter((doc): doc is { slug: string; frontmatter: DocFrontmatter } => doc !== null)
    .sort((a, b) => (a.frontmatter.order || 999) - (b.frontmatter.order || 999));
}
