import { NextResponse } from "next/server";
import fs from "fs";
import path from "path";
import matter from "gray-matter";

interface DocMetadata {
  title: string;
  description: string;
  order?: number;
}

interface SearchDoc {
  title: string;
  description: string;
  slug: string;
  content: string;
}

function getAllDocs(): SearchDoc[] {
  const docsDirectory = path.join(process.cwd(), "content/docs");
  const docs: SearchDoc[] = [];

  function readDocsRecursive(dir: string, baseSlug = "") {
    const files = fs.readdirSync(dir);

    files.forEach((file) => {
      const filePath = path.join(dir, file);
      const stat = fs.statSync(filePath);

      if (stat.isDirectory()) {
        readDocsRecursive(filePath, `${baseSlug}/${file}`);
      } else if (file.endsWith(".mdx") || file.endsWith(".md")) {
        const fileContents = fs.readFileSync(filePath, "utf8");
        const { data, content } = matter(fileContents);
        const metadata = data as DocMetadata;

        const slug = file === "index.mdx" || file === "index.md"
          ? baseSlug || "/"
          : `${baseSlug}/${file.replace(/\.(mdx|md)$/, "")}`;

        // Remove markdown syntax for cleaner search
        const cleanContent = content
          .replace(/```[\s\S]*?```/g, "") // Remove code blocks
          .replace(/`[^`]+`/g, "") // Remove inline code
          .replace(/[#*_~]/g, "") // Remove markdown formatting
          .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1") // Convert links to text
          .substring(0, 500); // Limit content length

        docs.push({
          title: metadata.title || file.replace(/\.(mdx|md)$/, ""),
          description: metadata.description || "",
          slug: slug.startsWith("/") ? slug : `/${slug}`,
          content: cleanContent,
        });
      }
    });
  }

  try {
    readDocsRecursive(docsDirectory);
  } catch (error) {
    console.error("Error reading docs:", error);
  }

  return docs;
}

export async function GET() {
  try {
    const docs = getAllDocs();
    return NextResponse.json({ docs });
  } catch (error) {
    console.error("Search API error:", error);
    return NextResponse.json({ docs: [], error: "Failed to fetch documentation" }, { status: 500 });
  }
}
