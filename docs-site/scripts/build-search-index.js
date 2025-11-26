/**
 * Build-time script to generate search index for documentation.
 * Run this before `next build` to create a static search index.
 */

const fs = require('fs');
const path = require('path');
const matter = require('gray-matter');

const DOCS_DIR = path.join(process.cwd(), 'content/docs');
const OUTPUT_FILE = path.join(process.cwd(), 'public/search-index.json');

function getAllDocs() {
  const docs = [];

  function readDocsRecursive(dir, baseSlug = '') {
    const files = fs.readdirSync(dir);

    files.forEach((file) => {
      const filePath = path.join(dir, file);
      const stat = fs.statSync(filePath);

      if (stat.isDirectory()) {
        readDocsRecursive(filePath, `${baseSlug}/${file}`);
      } else if (file.endsWith('.mdx') || file.endsWith('.md')) {
        const fileContents = fs.readFileSync(filePath, 'utf8');
        const { data, content } = matter(fileContents);

        const slug = file === 'index.mdx' || file === 'index.md'
          ? baseSlug || '/'
          : `${baseSlug}/${file.replace(/\.(mdx|md)$/, '')}`;

        // Clean content for search - remove code blocks, links, formatting
        const cleanContent = content
          .replace(/```[\s\S]*?```/g, ' ') // Remove code blocks
          .replace(/`[^`]+`/g, (match) => match.slice(1, -1)) // Keep inline code text
          .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1') // Convert links to text
          .replace(/#{1,6}\s*/g, '') // Remove heading markers
          .replace(/[*_~]/g, '') // Remove bold/italic/strikethrough
          .replace(/>\s*/g, '') // Remove blockquotes
          .replace(/\|[^|]+\|/g, ' ') // Simplify tables
          .replace(/-{3,}/g, ' ') // Remove horizontal rules
          .replace(/\n+/g, ' ') // Normalize newlines
          .replace(/\s+/g, ' ') // Normalize whitespace
          .trim();

        // Extract headings for better search context
        const headings = [];
        const headingRegex = /^#{1,6}\s+(.+)$/gm;
        let match;
        while ((match = headingRegex.exec(content)) !== null) {
          headings.push(match[1].replace(/[*_`]/g, ''));
        }

        docs.push({
          id: docs.length,
          title: data.title || file.replace(/\.(mdx|md)$/, ''),
          description: data.description || '',
          slug: slug.startsWith('/') ? slug : `/${slug}`,
          content: cleanContent.substring(0, 2000), // More content for better search
          headings: headings.join(' '),
          category: baseSlug.split('/')[1] || 'general',
        });
      }
    });
  }

  try {
    readDocsRecursive(DOCS_DIR);
  } catch (error) {
    console.error('Error reading docs:', error);
  }

  return docs;
}

function buildSearchIndex() {
  console.log('Building search index...');

  const docs = getAllDocs();

  // Create search index data
  const searchIndex = {
    version: 2,
    generated: new Date().toISOString(),
    totalDocs: docs.length,
    docs: docs.map(doc => ({
      id: doc.id,
      title: doc.title,
      description: doc.description,
      slug: doc.slug,
      content: doc.content,
      headings: doc.headings,
      category: doc.category,
    })),
  };

  // Ensure public directory exists
  const publicDir = path.dirname(OUTPUT_FILE);
  if (!fs.existsSync(publicDir)) {
    fs.mkdirSync(publicDir, { recursive: true });
  }

  // Write search index
  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(searchIndex));

  console.log(`Search index built successfully!`);
  console.log(`  - Total documents: ${docs.length}`);
  console.log(`  - Output: ${OUTPUT_FILE}`);
  console.log(`  - Size: ${(fs.statSync(OUTPUT_FILE).size / 1024).toFixed(2)} KB`);
}

buildSearchIndex();
