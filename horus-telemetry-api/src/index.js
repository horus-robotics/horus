/**
 * HORUS Anonymous Telemetry API
 * Cloudflare Worker for collecting anonymous installation statistics
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

    // POST /telemetry - Receive telemetry event
    if (url.pathname === '/telemetry' && request.method === 'POST') {
      try {
        const data = await request.json();

        // Validate required fields
        const required = ['event', 'status', 'version', 'install_id', 'os', 'arch', 'timestamp'];
        for (const field of required) {
          if (!data[field]) {
            return new Response(JSON.stringify({
              error: `Missing required field: ${field}`
            }), {
              status: 400,
              headers: { ...corsHeaders, 'Content-Type': 'application/json' }
            });
          }
        }

        // Insert into database
        await env.DB.prepare(
          'INSERT INTO events (event, status, version, install_id, os, arch, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)'
        ).bind(
          data.event,
          data.status,
          data.version,
          data.install_id,
          data.os,
          data.arch,
          data.timestamp
        ).run();

        return new Response(JSON.stringify({ success: true }), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });

      } catch (error) {
        console.error('Error storing telemetry:', error);
        return new Response(JSON.stringify({
          error: 'Internal server error',
          message: error.message
        }), {
          status: 500,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });
      }
    }

    // GET /telemetry/badge - Shields.io badge endpoint
    if (url.pathname === '/telemetry/badge' && request.method === 'GET') {
      try {
        // Count unique successful installations
        const result = await env.DB.prepare(
          'SELECT COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ?'
        ).bind('install', 'success').first();

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

    // GET /telemetry/stats - Public stats dashboard
    if (url.pathname === '/telemetry/stats' && request.method === 'GET') {
      try {
        const stats = {};

        // Total unique installations
        const totalInstalls = await env.DB.prepare(
          'SELECT COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ?'
        ).bind('install', 'success').first();
        stats.total_installations = totalInstalls?.count || 0;

        // Platform breakdown
        const platforms = await env.DB.prepare(
          'SELECT os, COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ? GROUP BY os'
        ).bind('install', 'success').all();
        stats.platforms = platforms.results || [];

        // Architecture breakdown
        const architectures = await env.DB.prepare(
          'SELECT arch, COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ? GROUP BY arch'
        ).bind('install', 'success').all();
        stats.architectures = architectures.results || [];

        // Version distribution (top 10)
        const versions = await env.DB.prepare(
          'SELECT version, COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ? GROUP BY version ORDER BY count DESC LIMIT 10'
        ).bind('install', 'success').all();
        stats.versions = versions.results || [];

        // Success rate
        const totalAttempts = await env.DB.prepare(
          'SELECT COUNT(*) as count FROM events WHERE event = ?'
        ).bind('install').first();

        const successfulAttempts = await env.DB.prepare(
          'SELECT COUNT(*) as count FROM events WHERE event = ? AND status = ?'
        ).bind('install', 'success').first();

        if (totalAttempts?.count > 0) {
          const rate = (successfulAttempts?.count || 0) / totalAttempts.count * 100;
          stats.install_success_rate = rate.toFixed(2) + '%';
        } else {
          stats.install_success_rate = 'N/A';
        }

        // Recent activity (last 7 days)
        const sevenDaysAgo = Math.floor(Date.now() / 1000) - (7 * 24 * 60 * 60);
        const recentInstalls = await env.DB.prepare(
          'SELECT COUNT(DISTINCT install_id) as count FROM events WHERE event = ? AND status = ? AND timestamp > ?'
        ).bind('install', 'success', sevenDaysAgo).first();
        stats.installs_last_7_days = recentInstalls?.count || 0;

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
        name: 'HORUS Telemetry API',
        version: '1.0.0',
        endpoints: {
          'POST /telemetry': 'Submit telemetry event',
          'GET /telemetry/badge': 'Get installation count badge (Shields.io format)',
          'GET /telemetry/stats': 'Get public statistics'
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
