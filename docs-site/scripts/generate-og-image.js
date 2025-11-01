#!/usr/bin/env node

/**
 * Generate OG image for social media previews
 * Run with: node scripts/generate-og-image.js
 *
 * This creates a simple SVG that can be converted to PNG using:
 * - Browser screenshot
 * - ImageMagick: convert og-image.svg og-image.png
 * - Online converter: https://cloudconvert.com/svg-to-png
 */

const fs = require('fs');
const path = require('path');

const svg = `<svg width="1200" height="630" xmlns="http://www.w3.org/2000/svg">
  <!-- Background -->
  <rect width="1200" height="630" fill="#16181c"/>

  <!-- Grid pattern -->
  <defs>
    <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
      <path d="M 40 0 L 0 0 0 40" fill="none" stroke="#1e2228" stroke-width="1"/>
    </pattern>
  </defs>
  <rect width="1200" height="630" fill="url(#grid)" opacity="0.3"/>

  <!-- Accent line -->
  <rect x="0" y="0" width="8" height="630" fill="#00d9ff"/>

  <!-- Content -->
  <g transform="translate(80, 200)">
    <!-- Logo/Title -->
    <text x="0" y="0" font-family="monospace" font-size="120" font-weight="bold" fill="#ffffff">
      HORUS
    </text>

    <!-- Subtitle -->
    <text x="0" y="80" font-family="monospace" font-size="36" fill="#8a9199">
      Ultra-Low Latency IPC for Robotics
    </text>

    <!-- Key stats -->
    <g transform="translate(0, 160)">
      <rect x="0" y="0" width="200" height="80" fill="#1e2228" rx="8"/>
      <text x="100" y="30" font-family="monospace" font-size="32" font-weight="bold" fill="#00d9ff" text-anchor="middle">
        29ns
      </text>
      <text x="100" y="60" font-family="monospace" font-size="18" fill="#8a9199" text-anchor="middle">
        Latency
      </text>
    </g>

    <g transform="translate(230, 160)">
      <rect x="0" y="0" width="200" height="80" fill="#1e2228" rx="8"/>
      <text x="100" y="30" font-family="monospace" font-size="32" font-weight="bold" fill="#00d9ff" text-anchor="middle">
        Zero-Copy
      </text>
      <text x="100" y="60" font-family="monospace" font-size="18" fill="#8a9199" text-anchor="middle">
        Shared Memory
      </text>
    </g>

    <g transform="translate(460, 160)">
      <rect x="0" y="0" width="200" height="80" fill="#1e2228" rx="8"/>
      <text x="100" y="30" font-family="monospace" font-size="32" font-weight="bold" fill="#00d9ff" text-anchor="middle">
        Real-time
      </text>
      <text x="100" y="60" font-family="monospace" font-size="18" fill="#8a9199" text-anchor="middle">
        Control
      </text>
    </g>
  </g>

  <!-- Footer -->
  <text x="80" y="590" font-family="monospace" font-size="24" fill="#4a5158">
    docs.horus.dev
  </text>
</svg>`;

const outputPath = path.join(__dirname, '../public/og-image.svg');
fs.writeFileSync(outputPath, svg);
console.log('✓ Generated og-image.svg');
console.log('\nTo convert to PNG (1200x630):');
console.log('  Option 1: Use ImageMagick');
console.log('    convert -background none -size 1200x630 og-image.svg og-image.png');
console.log('  Option 2: Use online converter');
console.log('    https://cloudconvert.com/svg-to-png');
console.log('  Option 3: Open in browser and screenshot at exact size');
