"use client";

import Link from "next/link";
import { FiGithub } from "react-icons/fi";

export function DocsNav() {
  return (
    <nav className="sticky top-0 z-50 w-full border-b border-[var(--border)] bg-[rgba(22,24,28,0.95)] backdrop-blur-lg">
      <div className="w-full px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between relative">
          <div className="flex items-center gap-6">
            <Link
              href="/"
              className="flex items-center gap-2 font-bold text-xl bg-gradient-to-r from-[var(--accent)] to-[var(--success)] bg-clip-text text-transparent hover:opacity-80 transition-opacity"
            >
              HORUS <span className="text-sm font-normal text-[var(--text-secondary)]">/ docs</span>
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

          <div className="flex items-center gap-4">
            <Link
              href="/benchmarks"
              className="text-sm px-3 py-1.5 bg-[var(--surface)] border border-[var(--border)] rounded-md text-[var(--text-secondary)] hover:text-[var(--accent)] hover:border-[var(--accent)] transition-colors"
            >
              Benchmarks
            </Link>
            <a
              href="https://github.com/horus-robotics/horus"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors"
              title="GitHub Repository"
            >
              <FiGithub className="w-5 h-5" />
            </a>
          </div>
        </div>
      </div>
    </nav>
  );
}
