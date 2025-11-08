#!/usr/bin/env node

/**
 * Generate favicon from HORUS logo
 * Run with: node scripts/generate-favicons.js
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const publicDir = path.join(__dirname, '../public');
const logoPath = path.join(publicDir, 'horus_logo.png');

if (!fs.existsSync(logoPath)) {
  console.error('Error: horus_logo.png not found in public/');
  process.exit(1);
}

console.log('Generating favicons from HORUS logo...');

// Change to public directory
process.chdir(publicDir);

const sizes = [
  { size: 16, name: 'favicon-16x16.png' },
  { size: 32, name: 'favicon-32x32.png' },
  { size: 180, name: 'apple-touch-icon.png' },
];

try {
  sizes.forEach(({ size, name }) => {
    execSync(`convert horus_logo.png -resize ${size}x${size} ${name}`);
    const stats = fs.statSync(path.join(publicDir, name));
    console.log(` Generated ${name} (${Math.round(stats.size / 1024)}KB)`);
  });

  // Generate .ico file
  execSync(`convert horus_logo.png -resize 32x32 favicon.ico`);
  const icoStats = fs.statSync(path.join(publicDir, 'favicon.ico'));
  console.log(` Generated favicon.ico (${Math.round(icoStats.size / 1024)}KB)`);

  console.log('\n All favicons generated successfully!');
} catch (error) {
  console.error('Failed to generate favicons:', error.message);
  process.exit(1);
}
