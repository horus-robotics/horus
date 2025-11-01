#!/usr/bin/env node

/**
 * Generate OG image with real HORUS logo
 * Run with: node scripts/generate-og-image.js
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const publicDir = path.join(__dirname, '../public');
const logoPath = path.join(publicDir, 'horus_logo.png');
const ogImagePath = path.join(publicDir, 'og-image.png');

if (!fs.existsSync(logoPath)) {
  console.error('Error: horus_logo.png not found in public/');
  process.exit(1);
}

console.log('Generating OG image with HORUS logo...');

try {
  // Change to public directory and run convert
  process.chdir(publicDir);

  execSync(`convert -size 1200x630 xc:"#16181c" ` +
    `-fill "#00d9ff" -draw "rectangle 0,0 8,630" ` +
    `horus_logo.png -resize 500x500 -gravity center -geometry +0-50 -composite ` +
    `-font DejaVu-Sans-Mono-Bold -pointsize 48 -fill "#ffffff" -gravity south -annotate +0+100 "HORUS Documentation" ` +
    `-font DejaVu-Sans-Mono -pointsize 28 -fill "#00d9ff" -gravity south -annotate +0+50 "29ns Latency • Real-time Robotics" ` +
    `og-image.png`
  );

  const stats = fs.statSync(ogImagePath);
  console.log('✓ Generated og-image.png with logo');
  console.log(`  Size: ${Math.round(stats.size / 1024)}KB`);
  console.log('  Dimensions: 1200x630');
} catch (error) {
  console.error('Failed to generate OG image:', error.message);
  process.exit(1);
}
