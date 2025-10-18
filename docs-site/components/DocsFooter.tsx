export function DocsFooter() {
  return (
    <footer className="border-t border-[var(--border)] bg-[var(--surface)] mt-20">
      <div className="w-full px-8 lg:px-16 py-16">
        <div className="grid grid-cols-1 md:grid-cols-12 gap-12">
          {/* Brand Section - Takes 4 columns */}
          <div className="md:col-span-4">
            <h3 className="font-bold text-2xl bg-gradient-to-r from-[var(--accent)] to-[var(--success)] bg-clip-text text-transparent mb-3">
              HORUS
            </h3>
            <p className="text-sm text-[var(--text-secondary)] leading-relaxed max-w-xs">
              Hybrid Optimized Robotics Unified System. Ultra-low latency IPC for real-time robotics.
            </p>
          </div>

          {/* Links Sections - Takes 8 columns, divided into 3 */}
          <div className="md:col-span-8 grid grid-cols-1 sm:grid-cols-3 gap-8">
            <div>
              <h4 className="font-semibold text-[var(--text-primary)] mb-3 text-sm uppercase tracking-wider">Documentation</h4>
              <ul className="space-y-2.5 text-sm text-[var(--text-secondary)]">
                <li><a href="/getting-started" className="hover:text-[var(--accent)] transition-colors">Getting Started</a></li>
                <li><a href="/cli-reference" className="hover:text-[var(--accent)] transition-colors">CLI Reference</a></li>
                <li><a href="/node-macro" className="hover:text-[var(--accent)] transition-colors">Node Macro</a></li>
                <li><a href="/parameters" className="hover:text-[var(--accent)] transition-colors">Parameters</a></li>
                <li><a href="/dashboard" className="hover:text-[var(--accent)] transition-colors">Dashboard</a></li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold text-[var(--text-primary)] mb-3 text-sm uppercase tracking-wider">Resources</h4>
              <ul className="space-y-2.5 text-sm text-[var(--text-secondary)]">
                <li><a href="https://github.com/neos-builder/horus" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">GitHub Repository</a></li>
                <li><a href="https://marketplace.horus-registry.dev/" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">Package Marketplace</a></li>
                <li><a href="/benchmarks" className="hover:text-[var(--accent)] transition-colors">Benchmarks</a></li>
                <li><a href="/examples" className="hover:text-[var(--accent)] transition-colors">Examples</a></li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold text-[var(--text-primary)] mb-3 text-sm uppercase tracking-wider">Community</h4>
              <ul className="space-y-2.5 text-sm text-[var(--text-secondary)]">
                <li><a href="https://github.com/neos-builder/horus/discussions" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">Discussions</a></li>
                <li><a href="https://github.com/neos-builder/horus/issues" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">Issues</a></li>
                <li><a href="/goals" className="hover:text-[var(--accent)] transition-colors">Goals & Vision</a></li>
                <li><a href="/architecture" className="hover:text-[var(--accent)] transition-colors">Architecture</a></li>
              </ul>
            </div>
          </div>
        </div>

        <div className="mt-12 pt-8 border-t border-[var(--border)] flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
          <div className="text-sm text-[var(--text-tertiary)]">
            Copyright {new Date().getFullYear()} HORUS Contributors. Apache-2.0 License.
          </div>
          <div className="flex gap-6 text-sm text-[var(--text-secondary)]">
            <a href="https://github.com/neos-builder/horus" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">GitHub</a>
            <a href="/performance" className="hover:text-[var(--accent)] transition-colors">Performance</a>
            <a href="https://github.com/neos-builder/horus/blob/main/LICENSE" target="_blank" rel="noopener noreferrer" className="hover:text-[var(--accent)] transition-colors">License</a>
          </div>
        </div>
      </div>
    </footer>
  );
}
