# Contributing to HORUS Documentation

Thank you for your interest in contributing to the HORUS documentation! This guide will help you get started.

## 🌟 How to Contribute

### Documentation Improvements

1. **Fork the repository**
   ```bash
   git clone https://github.com/horus-robotics/horus
   cd horus/HORUS/docs-site
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Run locally**
   ```bash
   npm run dev
   # Visit http://localhost:3002
   ```

4. **Make your changes**
   - Edit `.mdx` files in `content/`
   - Add new pages as needed
   - Update navigation in components

5. **Test thoroughly**
   - Verify all links work
   - Check code examples
   - Test on mobile/desktop
   - Run `npm run build` to check for errors

6. **Submit a pull request**
   - Clear description of changes
   - Reference any related issues
   - Include screenshots if UI changes

## 📝 Content Guidelines

### Writing Style

- **Clear and concise** - Get to the point quickly
- **Beginner-friendly** - Explain concepts clearly
- **Code-focused** - Show examples, not just theory
- **Accurate** - Test all code snippets

### Code Examples

```rust
// ✅ Good: Complete, runnable example
use horus::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut node = NodeBuilder::new()
        .name("example")
        .build()?;
    Ok(())
}
```

```rust
// ❌ Bad: Incomplete snippet
let mut node = ...;
// do something
```

### Performance Numbers

When documenting performance:

- Use **production benchmark results** (366ns-2.8μs range)
- Include **message types** (CmdVel, LaserScan, IMU)
- Show **comparisons to ROS2** (100-270x faster)
- Link to `/docs/benchmarks` for details

## 🎨 Formatting

### Markdown/MDX

- Use headings hierarchically (h1 → h2 → h3)
- Add code language tags (```rust, ```bash, ```toml)
- Include alt text for images
- Use tables for comparisons

### Frontmatter

Every `.mdx` file should have:

```yaml
---
title: Page Title
description: Brief description for SEO
order: 1  # For sidebar ordering
---
```

## 🔍 What to Document

### High Priority

- Getting started guides
- Common use cases
- Error solutions
- Performance optimization
- Migration guides

### Always Welcome

- Real-world examples
- Best practices
- Troubleshooting tips
- API clarifications
- Diagrams and visualizations

## 🚫 What NOT to Include

- **No proprietary features** - Only document open-source HORUS framework
- **No marketplace references** - Marketplace is separate (not OSS)
- **No IDE features** - horus_ide is outside HORUS/ (not OSS)
- **No private APIs** - Only public, documented features
- **No outdated performance** - Use current production benchmarks

## 📦 Adding New Pages

1. Create `.mdx` file in appropriate `content/` directory
2. Add frontmatter with title, description, order
3. Write content following guidelines
4. Update navigation in `components/DocsSidebar.tsx` if needed
5. Add link in `app/page.tsx` if it's a major section
6. Test that it renders correctly

## 🐛 Reporting Issues

Found a problem with the docs?

1. **Check existing issues** - Someone may have reported it
2. **Open a new issue** - Be specific about the problem
3. **Suggest a fix** - Even better, submit a PR!

## 💡 Ideas for Contribution

### Beginner-Friendly

- Fix typos and grammar
- Improve code examples
- Add clarifying comments
- Update broken links

### Intermediate

- Write tutorial guides
- Add diagrams/visualizations
- Improve navigation
- Add search functionality

### Advanced

- Restructure documentation
- Add interactive examples
- Create video tutorials
- Integrate with CI/CD

## ✅ Review Process

1. **Automated checks** - Build must pass
2. **Content review** - Accuracy and clarity
3. **Technical review** - Code examples work
4. **Maintainer approval** - Final approval

Expect 1-3 days for review. We appreciate your patience!

## 📄 License

By contributing, you agree that your contributions will be dual-licensed under MIT/Apache-2.0, matching the HORUS framework license.

## 🤝 Code of Conduct

Be respectful, inclusive, and constructive. We're building this together!

## 📧 Questions?

- **GitHub Discussions**: https://github.com/horus-robotics/horus/discussions
- **GitHub Issues**: https://github.com/horus-robotics/horus/issues

---

**Thank you for contributing to HORUS!** 🚀
