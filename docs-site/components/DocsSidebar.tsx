"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FiChevronDown, FiChevronRight, FiX } from "react-icons/fi";
import { useState, useEffect } from "react";

interface DocLink {
  title: string;
  href: string;
  order?: number;
}

interface SidebarSection {
  title: string;
  links: DocLink[];
}

const sections: SidebarSection[] = [
  {
    title: "Getting Started",
    links: [
      { title: "Introduction", href: "/getting-started", order: 1 },
      { title: "Installation", href: "/getting-started/installation", order: 2 },
      { title: "Quick Start", href: "/getting-started/quick-start", order: 3 },
      { title: "node! Macro", href: "/node-macro", order: 4 },
    ],
  },
  {
    title: "Core Concepts",
    links: [
      { title: "Overview", href: "/core", order: 1 },
      { title: "Nodes", href: "/core-concepts-nodes", order: 2 },
      { title: "Hub (MPMC)", href: "/core-concepts-hub", order: 3 },
      { title: "Scheduler", href: "/core-concepts-scheduler", order: 4 },
      { title: "Shared Memory", href: "/core-concepts-shared-memory", order: 5 },
    ],
  },
  {
    title: "Guides",
    links: [
      { title: "Dashboard", href: "/dashboard", order: 1 },
      { title: "Parameters", href: "/parameters", order: 2 },
      { title: "CLI Reference", href: "/cli-reference", order: 3 },
      { title: "Package Management", href: "/package-management", order: 4 },
      { title: "Environment Management", href: "/environment-management", order: 5 },
      { title: "Marketplace & Registry", href: "/marketplace", order: 6 },
      { title: "Authentication", href: "/authentication", order: 7 },
      { title: "Remote Deployment", href: "/remote-deployment", order: 8 },
      { title: "Library Reference", href: "/library-reference", order: 9 },
      { title: "Message Types", href: "/message-types", order: 10 },
      { title: "Examples", href: "/examples", order: 11 },
      { title: "Performance", href: "/performance", order: 12 },
      { title: "Python Bindings", href: "/python-bindings", order: 13 },
      { title: "C Bindings", href: "/c-bindings", order: 14 },
    ],
  },
  {
    title: "API Reference",
    links: [
      { title: "Node", href: "/api-node", order: 1 },
      { title: "Hub", href: "/api-hub", order: 2 },
      { title: "Scheduler", href: "/api-scheduler", order: 3 },
    ],
  },
];

interface DocsSidebarProps {
  isOpen?: boolean;
  onClose?: () => void;
}

export function DocsSidebar({ isOpen = true, onClose }: DocsSidebarProps) {
  const pathname = usePathname();
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({
    "Getting Started": true,
    "Core Concepts": true,
    "Guides": true,
    "API Reference": true,
  });

  const toggleSection = (title: string) => {
    setExpandedSections((prev) => ({ ...prev, [title]: !prev[title] }));
  };

  // Close sidebar on mobile when clicking a link
  const handleLinkClick = () => {
    if (onClose) {
      onClose();
    }
  };

  // Prevent body scroll when mobile menu is open
  useEffect(() => {
    if (isOpen && onClose) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen, onClose]);

  const sidebarContent = (
    <div className="p-6 space-y-6 pb-12">
      {sections.map((section) => {
        const isExpanded = expandedSections[section.title];

        return (
          <div key={section.title}>
            <button
              onClick={() => toggleSection(section.title)}
              className="flex items-center gap-2 w-full text-left font-semibold text-[var(--text-primary)] hover:text-[var(--accent)] transition-colors mb-2 touch-manipulation"
            >
              {isExpanded ? (
                <FiChevronDown className="w-4 h-4" />
              ) : (
                <FiChevronRight className="w-4 h-4" />
              )}
              {section.title}
            </button>

            {isExpanded && (
              <ul className="space-y-1 ml-6">
                {section.links
                  .sort((a, b) => (a.order || 999) - (b.order || 999))
                  .map((link) => {
                    const isActive = pathname === link.href;

                    return (
                      <li key={link.href}>
                        <Link
                          href={link.href}
                          onClick={handleLinkClick}
                          className={`block px-3 py-2 rounded text-sm transition-colors touch-manipulation ${
                            isActive
                              ? "bg-[var(--accent)]/10 text-[var(--accent)] font-medium border-l-2 border-[var(--accent)]"
                              : "text-[var(--text-secondary)] hover:text-[var(--accent)] hover:bg-[var(--border)]"
                          }`}
                        >
                          {link.title}
                        </Link>
                      </li>
                    );
                  })}
              </ul>
            )}
          </div>
        );
      })}
    </div>
  );

  // Desktop sidebar
  if (!onClose) {
    return (
      <aside className="hidden lg:block w-64 border-r border-[var(--border)] bg-[var(--surface)] h-[calc(100vh-4rem)] sticky top-16 overflow-y-auto">
        {sidebarContent}
      </aside>
    );
  }

  // Mobile sidebar (drawer)
  return (
    <>
      {/* Backdrop */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 lg:hidden backdrop-blur-sm"
          onClick={onClose}
        />
      )}

      {/* Drawer */}
      <aside
        className={`fixed top-0 left-0 bottom-0 w-80 max-w-[85vw] bg-[var(--background)] border-r border-[var(--border)] z-50 lg:hidden transform transition-transform duration-300 ease-in-out overflow-y-auto ${
          isOpen ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        {/* Close button */}
        <div className="sticky top-0 bg-[var(--background)] border-b border-[var(--border)] p-4 flex items-center justify-between">
          <span className="font-semibold text-[var(--text-primary)]">Documentation</span>
          <button
            onClick={onClose}
            className="p-2 hover:bg-[var(--surface)] rounded-md transition-colors touch-manipulation"
            aria-label="Close menu"
          >
            <FiX className="w-5 h-5" />
          </button>
        </div>
        {sidebarContent}
      </aside>
    </>
  );
}
