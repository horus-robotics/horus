# HORUS Documentation Site

Open-source documentation for the HORUS robotics framework.

## 🌐 Overview

This is the official documentation site for HORUS - a production-grade, open-source robotics framework built in Rust. The site provides comprehensive guides, API references, and performance benchmarks.

## 🚀 Running Locally

```bash
# Install dependencies
npm install

# Start development server (port 3009)
npm run dev

# Build for production
npm run build

# Start production server
npm start
```

Visit `http://localhost:3009` to view the documentation.

## 📝 Content Structure

```
content/
├── docs/                              # Core documentation (30+ pages)
│   ├── getting-started.mdx
│   ├── installation.mdx
│   ├── quick-start.mdx
│   ├── node-macro.mdx
│   ├── dashboard.mdx
│   ├── parameters.mdx
│   ├── cli-reference.mdx
│   ├── package-management.mdx        # Package install/publish
│   ├── environment-management.mdx    # Freeze/restore environments
│   ├── marketplace.mdx                # Registry and marketplace
│   ├── authentication.mdx             # GitHub OAuth, API keys
│   ├── remote-deployment.mdx          # Deploy to robots
│   ├── library-reference.mdx          # Standard library components
│   ├── core-concepts-nodes.mdx
│   ├── core-concepts-hub.mdx
│   ├── core-concepts-scheduler.mdx
│   ├── core-concepts-shared-memory.mdx
│   ├── api-node.mdx                   # Node API reference
│   ├── api-hub.mdx                    # Hub API reference
│   ├── api-scheduler.mdx              # Scheduler API reference
│   ├── message-types.mdx
│   ├── examples.mdx
│   ├── performance.mdx
│   ├── multi-language.mdx             # Python & C bindings
│   └── architecture.mdx
└── assets/         # Images and media
```

### Documentation Categories

**Getting Started**
- Installation, Quick Start, node! Macro

**Core Concepts**
- Nodes, Hub (MPMC), Scheduler, Shared Memory

**Guides**
- Dashboard, Parameters, CLI Reference
- Package Management, Environment Management
- Marketplace & Registry, Authentication
- Remote Deployment, Library Reference
- Message Types, Examples, Performance, Multi-Language

**API Reference**
- Node, Hub, Scheduler APIs

## 🎨 Tech Stack

- **Next.js 14** - React framework with App Router
- **MDX** - Markdown with React components
- **Tailwind CSS** - Utility-first styling
- **Shiki** - Syntax highlighting
- **TypeScript** - Type safety

## 📦 Open Source

This documentation site is part of the HORUS open-source project:

- **License**: MIT/Apache-2.0 (dual-licensed)
- **Repository**: https://github.com/horus-robotics/horus
- **Framework**: `/HORUS` directory in the main repository

## 🤝 Contributing

We welcome contributions! To contribute to the documentation:

1. Fork the repository
2. Create a feature branch
3. Make your changes in `content/`
4. Test locally with `npm run dev`
5. Submit a pull request

### Writing Guidelines

- Use clear, concise language
- Include code examples
- Test all code snippets
- Follow existing formatting
- Update navigation if adding new pages

## 📊 Performance Focus

The documentation emphasizes HORUS's production-grade performance:

- **366ns-2.8μs** latency for real robotics messages
- **100-270x faster than ROS2**
- Production benchmarks with serde serialization
- Real-world message types (CmdVel, LaserScan, IMU, etc.)

## 🔗 Links

- **Main Repository**: https://github.com/horus-robotics/horus
- **Issues**: https://github.com/horus-robotics/horus/issues
- **Discussions**: https://github.com/horus-robotics/horus/discussions
- **Crates.io**: https://crates.io/search?q=horus

## 📄 License

Documentation content is dual-licensed under MIT/Apache-2.0, matching the HORUS framework license.

---

**Built with ❤️ by the open-source community**
