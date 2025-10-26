"use client";

import Link from "next/link";
import { FiGithub, FiMenu, FiSearch, FiMessageCircle } from "react-icons/fi";
import { ThemeToggle } from "./ThemeToggle";
import { SearchModal } from "./SearchModal";
import { useState, useEffect } from "react";

interface DocsNavProps {
  onMenuClick?: () => void;
}

export function DocsNav({ onMenuClick }: DocsNavProps) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);

  // Keyboard shortcut for search (Cmd+K / Ctrl+K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setIsSearchOpen(true);
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <nav className="sticky top-0 z-50 w-full border-b border-[var(--border)] bg-[var(--card-bg)] bg-opacity-95 backdrop-blur-lg">
      <div className="w-full px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between relative">
          <div className="flex items-center gap-3 sm:gap-6">
            {/* Hamburger menu for mobile */}
            {onMenuClick && (
              <button
                onClick={onMenuClick}
                className="lg:hidden p-2 hover:bg-[var(--surface)] rounded-md transition-colors touch-manipulation"
                aria-label="Open menu"
              >
                <FiMenu className="w-5 h-5" />
              </button>
            )}

            <Link
              href="/"
              className="flex items-center gap-2 font-bold text-lg sm:text-xl bg-gradient-to-r from-[var(--accent)] to-[var(--success)] bg-clip-text text-transparent hover:opacity-80 transition-opacity"
            >
              HORUS <span className="text-xs sm:text-sm font-normal text-[var(--text-secondary)]">/ docs</span>
            </Link>
            <div className="hidden md:flex items-center gap-6 ml-8">
              <Link
                href="/goals"
                className="text-sm text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors"
              >
                Goals & Vision
              </Link>
              <Link
                href="/getting-started"
                className="text-sm text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors"
              >
                Getting Started
              </Link>
            </div>
          </div>

          <div className="flex items-center gap-2 sm:gap-4">
            {/* Search Button */}
            <button
              onClick={() => setIsSearchOpen(true)}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-[var(--surface)] border border-[var(--border)] rounded-md text-[var(--text-secondary)] hover:border-[var(--accent)] transition-colors touch-manipulation"
              aria-label="Search documentation"
            >
              <FiSearch className="w-4 h-4" />
              <span className="hidden sm:inline">Search</span>
              <kbd className="hidden lg:inline-block ml-2 px-1.5 py-0.5 text-xs bg-[var(--card-bg)] border border-[var(--border)] rounded">
                ⌘K
              </kbd>
            </button>

            <a
              href="https://marketplace.horus-registry.dev/"
              target="_blank"
              rel="noopener noreferrer"
              className="hidden sm:block text-sm px-3 py-1.5 bg-gradient-to-r from-[var(--accent)] to-[var(--success)] text-white font-medium rounded-md hover:opacity-90 transition-opacity touch-manipulation"
            >
              Marketplace
            </a>
            <Link
              href="/benchmarks"
              className="hidden md:block text-sm px-3 py-1.5 bg-[var(--surface)] border border-[var(--border)] rounded-md text-[var(--text-secondary)] hover:text-[var(--accent)] hover:border-[var(--accent)] transition-colors touch-manipulation"
            >
              Benchmarks
            </Link>
            <ThemeToggle />
            <a
              href="https://discord.gg/hEZC3ev2Nf"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors touch-manipulation"
              title="Join Discord Community"
              aria-label="Discord Community"
            >
              <FiMessageCircle className="w-5 h-5" />
            </a>
            <a
              href="https://github.com/horus-robotics/horus"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors touch-manipulation"
              title="GitHub Repository"
              aria-label="GitHub Repository"
            >
              <FiGithub className="w-5 h-5" />
            </a>
          </div>
        </div>
      </div>

      {/* Search Modal */}
      <SearchModal isOpen={isSearchOpen} onClose={() => setIsSearchOpen(false)} />
    </nav>
  );
}
