import { MetadataRoute } from 'next';

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: 'HORUS Documentation',
    short_name: 'HORUS Docs',
    description: 'Ultra-low latency IPC framework for robotics and real-time control systems',
    start_url: '/',
    display: 'standalone',
    background_color: '#16181c',
    theme_color: '#16181c',
    icons: [
      {
        src: '/favicon-16x16.png',
        sizes: '16x16',
        type: 'image/png',
      },
      {
        src: '/favicon-32x32.png',
        sizes: '32x32',
        type: 'image/png',
      },
      {
        src: '/apple-touch-icon.png',
        sizes: '180x180',
        type: 'image/png',
      },
    ],
  };
}
