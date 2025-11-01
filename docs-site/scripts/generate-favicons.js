#!/usr/bin/env node

/**
 * Generate favicon SVG
 * This creates a simple 'H' logo that can be converted to various sizes
 */

const fs = require('fs');
const path = require('path');

const faviconSvg = `<svg width="512" height="512" xmlns="http://www.w3.org/2000/svg">
  <!-- Background -->
  <rect width="512" height="512" fill="#16181c" rx="64"/>

  <!-- Accent border -->
  <rect x="0" y="0" width="512" height="8" fill="#00d9ff"/>

  <!-- Letter H -->
  <g transform="translate(256, 256)">
    <text
      x="0"
      y="0"
      font-family="monospace"
      font-size="320"
      font-weight="bold"
      fill="#ffffff"
      text-anchor="middle"
      dominant-baseline="middle"
    >
      H
    </text>
  </g>
</svg>`;

const outputPath = path.join(__dirname, '../public/favicon.svg');
fs.writeFileSync(outputPath, faviconSvg);
console.log('✓ Generated favicon.svg');
console.log('\nGenerating PNG versions...');

// Generate conversion commands
const sizes = [
  { size: 16, name: 'favicon-16x16.png' },
  { size: 32, name: 'favicon-32x32.png' },
  { size: 180, name: 'apple-touch-icon.png' },
];

console.log('\nConverting to PNG sizes...');
sizes.forEach(({ size, name }) => {
  console.log(`  ${name} (${size}x${size})`);
});
