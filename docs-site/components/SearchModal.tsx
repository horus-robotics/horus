"use client";

import { useState, useEffect, useCallback } from "react";
import { FiSearch, FiX } from "react-icons/fi";
import Link from "next/link";

interface SearchResult {
  title: string;
  description: string;
  slug: string;
  content: string;
}

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);

  // Close on escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener("keydown", handleEscape);
      document.body.style.overflow = "hidden";
    }

    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = "unset";
    };
  }, [isOpen, onClose]);

  // Search function
  const performSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim()) {
      setResults([]);
      return;
    }

    setLoading(true);

    try {
      // Fetch search index
      const response = await fetch("/api/search");
      const data = await response.json();

      // Simple client-side fuzzy search
      const searchTerms = searchQuery.toLowerCase().split(" ");
      const filtered = data.docs.filter((doc: SearchResult) => {
        const searchText = `${doc.title} ${doc.description} ${doc.content}`.toLowerCase();
        return searchTerms.some(term => searchText.includes(term));
      });

      setResults(filtered.slice(0, 10));
    } catch (error) {
      console.error("Search error:", error);
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  // Debounce search
  useEffect(() => {
    const timer = setTimeout(() => {
      performSearch(query);
    }, 300);

    return () => clearTimeout(timer);
  }, [query, performSearch]);

  const handleResultClick = () => {
    onClose();
    setQuery("");
    setResults([]);
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-black/50 backdrop-blur-sm pt-20"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl mx-4 bg-[var(--card-bg)] border border-[var(--border)] rounded-lg shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search Input */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)]">
          <FiSearch className="w-5 h-5 text-[var(--text-secondary)]" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search documentation..."
            className="flex-1 bg-transparent outline-none text-[var(--text-primary)] placeholder:text-[var(--text-tertiary)]"
            autoFocus
          />
          <button
            onClick={onClose}
            className="p-1 hover:bg-[var(--surface)] rounded transition-colors"
            aria-label="Close search"
          >
            <FiX className="w-5 h-5 text-[var(--text-secondary)]" />
          </button>
        </div>

        {/* Results */}
        <div className="max-h-[60vh] overflow-y-auto">
          {loading && (
            <div className="p-8 text-center text-[var(--text-secondary)]">
              Searching...
            </div>
          )}

          {!loading && query && results.length === 0 && (
            <div className="p-8 text-center text-[var(--text-secondary)]">
              No results found for "{query}"
            </div>
          )}

          {!loading && results.length > 0 && (
            <div className="py-2">
              {results.map((result, index) => (
                <Link
                  key={index}
                  href={result.slug}
                  onClick={handleResultClick}
                  className="block px-4 py-3 hover:bg-[var(--surface)] transition-colors border-b border-[var(--border)] last:border-b-0"
                >
                  <div className="font-medium text-[var(--text-primary)] mb-1">
                    {result.title}
                  </div>
                  {result.description && (
                    <div className="text-sm text-[var(--text-secondary)] line-clamp-2">
                      {result.description}
                    </div>
                  )}
                </Link>
              ))}
            </div>
          )}

          {!loading && !query && (
            <div className="p-8 text-center text-[var(--text-secondary)]">
              <div className="mb-2">Start typing to search...</div>
              <div className="text-sm text-[var(--text-tertiary)]">
                Try searching for "getting started", "node", "scheduler", etc.
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-4 py-2 border-t border-[var(--border)] flex items-center justify-between text-xs text-[var(--text-tertiary)]">
          <div className="flex items-center gap-4">
            <span>Press <kbd className="px-1.5 py-0.5 bg-[var(--surface)] border border-[var(--border)] rounded">ESC</kbd> to close</span>
          </div>
          <div>{results.length > 0 && `${results.length} result${results.length === 1 ? '' : 's'}`}</div>
        </div>
      </div>
    </div>
  );
}
