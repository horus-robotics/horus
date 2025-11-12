"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FiChevronDown, FiChevronRight, FiX } from "react-icons/fi";
import { useState, useEffect } from "react";

interface DocLink {
  title: string;
  href: string;
  order?: number;
  children?: DocLink[];
}

interface SidebarSection {
  title: string;
  links: DocLink[];
}

const sections: SidebarSection[] = [
  {
    title: "Getting Started",
    links: [
      { title: "What is HORUS?", href: "/what-is-horus", order: 0 },
      { title: "Goals & Vision", href: "/goals", order: 1 },
      { title: "Complete Beginner's Guide", href: "/complete-beginners-guide", order: 2 },
      { title: "Installation", href: "/getting-started/installation", order: 4 },
      { title: "Quick Start", href: "/getting-started/quick-start", order: 5 },
      { title: "Second Application", href: "/second-application", order: 6 },
      { title: "Architecture", href: "/architecture", order: 7 },
      { title: "Troubleshooting", href: "/troubleshooting", order: 8 },
      { title: "Runtime Errors", href: "/troubleshooting-runtime", order: 9 },
      { title: "Basic Examples", href: "/basic-examples", order: 11 },
      { title: "Advanced Examples", href: "/advanced-examples", order: 12 },
    ],
  },
  {
    title: "Core Concepts",
    links: [
      { title: "Overview", href: "/core", order: 1 },
      { title: "Nodes", href: "/core-concepts-nodes", order: 2 },
      { title: "Hub (MPMC)", href: "/core-concepts-hub", order: 3 },
      { title: "Link (SPSC)", href: "/core-concepts-link", order: 4 },
      { title: "Scheduler", href: "/core-concepts-scheduler", order: 5 },
      { title: "Shared Memory", href: "/core-concepts-shared-memory", order: 6 },
      { title: "node! Macro", href: "/node-macro", order: 7 },
      { title: "message! Macro", href: "/message-macro", order: 8 },
      { title: "Message Types", href: "/message-types", order: 9 },
      { title: "Real-Time Nodes", href: "/realtime-nodes", order: 10 },
    ],
  },
  {
    title: "Development",
    links: [
      { title: "CLI Reference", href: "/cli-reference", order: 1 },
      { title: "Dashboard", href: "/dashboard", order: 2 },
      { title: "Simulation", href: "/simulation", order: 3 },
      { title: "Testing", href: "/testing", order: 4 },
      { title: "Parameters", href: "/parameters", order: 5 },
      { title: "Library Reference", href: "/library-reference", order: 6 },
    ],
  },
  {
    title: "Package Management",
    links: [
      { title: "Package Management", href: "/package-management", order: 1 },
      { title: "Using Prebuilt Nodes", href: "/using-prebuilt-nodes", order: 2 },
      { title: "Environment Management", href: "/environment-management", order: 3 },
      { title: "Configuration Reference", href: "/configuration", order: 4 },
    ],
  },
  {
    title: "Multi-Language",
    links: [
      { title: "Overview", href: "/multi-language", order: 1 },
      { title: "Python Bindings", href: "/python-bindings", order: 2 },
      { title: "Python Message Library", href: "/python-message-library", order: 3 },
      { title: "C++ Bindings", href: "/cpp-bindings", order: 4 },
      { title: "AI API Integration", href: "/ai-integration", order: 5 },
    ],
  },
  {
    title: "Performance",
    links: [
      { title: "Optimization Guide", href: "/performance", order: 1 },
      {
        title: "Benchmarks",
        href: "/benchmarks",
        order: 2,
        children: [
          { title: "Methodology", href: "/benchmarks/methodology", order: 1 },
          { title: "Detailed Results", href: "/benchmarks/results", order: 2 },
          { title: "vs ROS2", href: "/benchmarks/comparison-ros2", order: 3 },
          { title: "vs Zenoh", href: "/benchmarks/comparison-zenoh", order: 4 },
        ]
      },
    ],
  },
  {
    title: "Advanced Topics",
    links: [
      { title: "RTOS Integration", href: "/rtos-integration", order: 1 },
    ],
  },
  {
    title: "API Reference",
    links: [
      { title: "Overview", href: "/api", order: 0 },
      { title: "Node", href: "/api-node", order: 1 },
      { title: "Hub", href: "/api-hub", order: 2 },
      { title: "Link", href: "/api-link", order: 3 },
      { title: "Scheduler", href: "/api-scheduler", order: 4 },
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
    "Development": true,
    "Package Management": true,
    "Multi-Language": true,
    "Performance": true,
    "Advanced Topics": true,
    "API Reference": true,
  });

  // Track expanded nested items (by href)
  const [expandedItems, setExpandedItems] = useState<Record<string, boolean>>({});

  const toggleSection = (title: string) => {
    setExpandedSections((prev) => ({ ...prev, [title]: !prev[title] }));
  };

  const toggleItem = (href: string) => {
    setExpandedItems((prev) => ({ ...prev, [href]: !prev[href] }));
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

  // Recursive component to render link with potential children
  const renderLink = (link: DocLink, depth: number = 0) => {
    const isActive = pathname === link.href;
    const hasChildren = link.children && link.children.length > 0;
    const isExpanded = expandedItems[link.href];

    return (
      <li key={link.href}>
        <div className="flex items-center">
          {hasChildren && (
            <button
              onClick={() => toggleItem(link.href)}
              className="p-1 hover:bg-[var(--surface)] rounded transition-colors touch-manipulation"
              aria-label={isExpanded ? "Collapse" : "Expand"}
            >
              {isExpanded ? (
                <FiChevronDown className="w-3 h-3 text-[var(--text-secondary)]" />
              ) : (
                <FiChevronRight className="w-3 h-3 text-[var(--text-secondary)]" />
              )}
            </button>
          )}
          <Link
            href={link.href}
            onClick={handleLinkClick}
            className={`flex-1 block px-3 py-2 rounded text-sm transition-colors touch-manipulation ${
              hasChildren ? "" : depth > 0 ? "ml-4" : ""
            } ${
              isActive
                ? "bg-[var(--accent)]/10 text-[var(--accent)] font-medium border-l-2 border-[var(--accent)]"
                : "text-[var(--text-secondary)] hover:text-[var(--accent)] hover:bg-[var(--border)]"
            }`}
          >
            {link.title}
          </Link>
        </div>

        {hasChildren && isExpanded && (
          <ul className="space-y-1 ml-6 mt-1">
            {link.children!
              .sort((a, b) => (a.order ?? 999) - (b.order ?? 999))
              .map((child) => renderLink(child, depth + 1))}
          </ul>
        )}
      </li>
    );
  };

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
                  .sort((a, b) => (a.order ?? 999) - (b.order ?? 999))
                  .map((link) => renderLink(link, 0))}
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
