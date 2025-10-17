"use client";

import { useState } from "react";
import { DocsSidebar } from "./DocsSidebar";
import { DocsNav } from "./DocsNav";

interface DocsLayoutProps {
  children: React.ReactNode;
}

export function DocsLayout({ children }: DocsLayoutProps) {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  return (
    <>
      <DocsNav onMenuClick={() => setIsMobileMenuOpen(true)} />
      <div className="flex">
        {/* Desktop sidebar - always rendered */}
        <DocsSidebar />

        {/* Mobile sidebar - drawer */}
        <DocsSidebar
          isOpen={isMobileMenuOpen}
          onClose={() => setIsMobileMenuOpen(false)}
        />

        {/* Main content */}
        {children}
      </div>
    </>
  );
}
