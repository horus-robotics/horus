/**
 * HORUS Silent Installation Counter API
 * Cloudflare Worker for counting anonymous installations
 *
 * New Format (v3.0):
 * - POST /count: {event: "install", os: "Linux", timestamp: 123}
 * - No UUID, no tracking, pure counting
 */

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // CORS headers for cross-origin requests
    const corsHeaders = {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type',
    };

    // Handle CORS preflight requests
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        status: 204,
        headers: corsHeaders
      });
    }

    // POST /count - Simple installation counter (v3.0)
    if (url.pathname === '/count' && request.method === 'POST') {
      try {
        const data = await request.json();

        // Validate required fields (minimal)
        if (!data.event || !data.os || !data.timestamp) {
          return new Response(JSON.stringify({
            error: 'Missing required field (event, os, or timestamp)'
          }), {
            status: 400,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' }
          });
        }

        // Insert into simple counter table
        await env.DB.prepare(
          'INSERT INTO install_counts (event, os, timestamp) VALUES (?, ?, ?)'
        ).bind(
          data.event,
          data.os,
          data.timestamp
        ).run();

        return new Response(JSON.stringify({ success: true }), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });

      } catch (error) {
        console.error('Error storing count:', error);
        // Silent failure - never break install.sh
        return new Response(JSON.stringify({ success: true }), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });
      }
    }

    // GET /count/badge - Shields.io badge endpoint
    if (url.pathname === '/count/badge' && request.method === 'GET') {
      try {
        // Count total installations
        const result = await env.DB.prepare(
          'SELECT COUNT(*) as count FROM install_counts WHERE event = ?'
        ).bind('install').first();

        const count = result?.count || 0;

        // Format number with commas
        const formattedCount = count.toLocaleString('en-US');

        // Determine badge color based on install count
        let color = 'blue';
        if (count > 1000) color = 'brightgreen';
        else if (count > 100) color = 'green';
        else if (count > 10) color = 'yellowgreen';

        // Return Shields.io JSON format
        return new Response(JSON.stringify({
          schemaVersion: 1,
          label: 'installations',
          message: formattedCount,
          color: color
        }), {
          status: 200,
          headers: {
            ...corsHeaders,
            'Content-Type': 'application/json',
            'Cache-Control': 'public, max-age=3600' // Cache for 1 hour
          }
        });

      } catch (error) {
        console.error('Error fetching badge data:', error);
        return new Response(JSON.stringify({
          schemaVersion: 1,
          label: 'installations',
          message: 'error',
          color: 'red'
        }), {
          status: 500,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });
      }
    }

    // GET /count/stats - Public stats dashboard
    if (url.pathname === '/count/stats' && request.method === 'GET') {
      try {
        const stats = {};

        // Total installations
        const totalInstalls = await env.DB.prepare(
          'SELECT COUNT(*) as count FROM install_counts WHERE event = ?'
        ).bind('install').first();
        stats.total_installations = totalInstalls?.count || 0;

        // Platform breakdown
        const platforms = await env.DB.prepare(
          'SELECT os, COUNT(*) as count FROM install_counts WHERE event = ? GROUP BY os'
        ).bind('install').all();
        stats.platforms = platforms.results || [];

        // Recent activity (last 7 days)
        const sevenDaysAgo = Math.floor(Date.now() / 1000) - (7 * 24 * 60 * 60);
        const recentInstalls = await env.DB.prepare(
          'SELECT COUNT(*) as count FROM install_counts WHERE event = ? AND timestamp > ?'
        ).bind('install', sevenDaysAgo).first();
        stats.installs_last_7_days = recentInstalls?.count || 0;

        // Installs per day (last 30 days)
        const thirtyDaysAgo = Math.floor(Date.now() / 1000) - (30 * 24 * 60 * 60);
        const dailyInstalls = await env.DB.prepare(
          `SELECT DATE(timestamp, 'unixepoch') as date, COUNT(*) as count
           FROM install_counts
           WHERE event = ? AND timestamp > ?
           GROUP BY date
           ORDER BY date DESC
           LIMIT 30`
        ).bind('install', thirtyDaysAgo).all();
        stats.daily_installs = dailyInstalls.results || [];

        return new Response(JSON.stringify(stats, null, 2), {
          status: 200,
          headers: {
            ...corsHeaders,
            'Content-Type': 'application/json',
            'Cache-Control': 'public, max-age=300' // Cache for 5 minutes
          }
        });

      } catch (error) {
        console.error('Error fetching stats:', error);
        return new Response(JSON.stringify({
          error: 'Internal server error',
          message: error.message
        }), {
          status: 500,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });
      }
    }

    // GET / - API info and health check
    if (url.pathname === '/' && request.method === 'GET') {
      return new Response(JSON.stringify({
        name: 'HORUS Installation Counter API',
        version: '3.0.0',
        description: 'Silent installation counting - no tracking, pure counts',
        endpoints: {
          'POST /count': 'Submit install count (event, os, timestamp)',
          'GET /count/badge': 'Get installation count badge (Shields.io format)',
          'GET /count/stats': 'Get public statistics'
        },
        status: 'operational'
      }, null, 2), {
        status: 200,
        headers: {
          ...corsHeaders,
          'Content-Type': 'application/json'
        }
      });
    }

    // 404 for unknown routes
    return new Response(JSON.stringify({
      error: 'Not Found',
      path: url.pathname
    }), {
      status: 404,
      headers: { ...corsHeaders, 'Content-Type': 'application/json' }
    });
  }
};
