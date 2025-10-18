"use client";

import Link from "next/link";
import { FiGithub, FiMenu } from "react-icons/fi";
import { ThemeToggle } from "./ThemeToggle";

interface DocsNavProps {
  onMenuClick?: () => void;
}

export function DocsNav({ onMenuClick }: DocsNavProps) {
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
              className="hidden sm:block text-sm px-3 py-1.5 bg-[var(--surface)] border border-[var(--border)] rounded-md text-[var(--text-secondary)] hover:text-[var(--accent)] hover:border-[var(--accent)] transition-colors touch-manipulation"
            >
              Benchmarks
            </Link>
            <ThemeToggle />
            <a
              href="https://github.com/neos-builder/horus"
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
    </nav>
  );
}
