#!/usr/bin/env node

/**
 * Node.js Function Runtime Server
 *
 * This HTTP server runs inside a Firecracker MicroVM and executes
 * serverless functions on demand. Each function gets its own VM.
 *
 * Endpoints:
 *   GET  /health       - Health check
 *   POST /invoke       - Execute the function
 *   POST /reload       - Hot-reload function code
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = process.env.PORT || 3000;
const FUNCTION_CODE_PATH = process.env.FUNCTION_CODE_PATH || '/function/code.js';
const FUNCTION_HANDLER = process.env.FUNCTION_HANDLER || 'handler';

let handler = null;
let loadError = null;

/**
 * Load (or reload) the function code
 */
function loadFunction() {
  try {
    // Clear require cache to enable hot-reloading
    delete require.cache[require.resolve(FUNCTION_CODE_PATH)];

    if (fs.existsSync(FUNCTION_CODE_PATH)) {
      const module = require(FUNCTION_CODE_PATH);
      handler = module[FUNCTION_HANDLER];

      if (typeof handler !== 'function') {
        throw new Error(`Handler "${FUNCTION_HANDLER}" is not a function`);
      }

      loadError = null;
      console.log(`[Runtime] Loaded function handler: ${FUNCTION_HANDLER}`);
      return true;
    } else {
      loadError = `Function code not found at ${FUNCTION_CODE_PATH}`;
      console.error(`[Runtime] ${loadError}`);
      return false;
    }
  } catch (error) {
    loadError = error.message;
    console.error(`[Runtime] Failed to load function: ${error.message}`);
    return false;
  }
}

/**
 * HTTP server request handler
 */
const server = http.createServer(async (req, res) => {
  // Health check endpoint
  if (req.url === '/health' && req.method === 'GET') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'healthy',
      handler: FUNCTION_HANDLER,
      codeLoaded: handler !== null,
      error: loadError,
    }));
    return;
  }

  // Reload endpoint
  if (req.url === '/write-code' && req.method === 'POST') {
    // Write new code to disk and reload
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });

    req.on('end', () => {
      try {
        const { code, handler: newHandler } = JSON.parse(body);

        if (!code) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Missing code field' }));
          return;
        }

        // Write code with module.exports
        const codeContent = `${code}\n\nmodule.exports = { ${newHandler || FUNCTION_HANDLER} };`;
        require('fs').writeFileSync(FUNCTION_CODE_PATH, codeContent);

        // Reload the function
        const success = loadFunction();

        res.writeHead(success ? 200 : 500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          success,
          error: loadError,
        }));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }

  if (req.url === '/reload' && req.method === 'POST') {
    const success = loadFunction();
    res.writeHead(success ? 200 : 500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      success,
      error: loadError,
    }));
    return;
  }

  // Invoke endpoint
  if (req.url === '/invoke' && req.method === 'POST') {
    // Check if function is loaded
    if (!handler) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        status: 'error',
        error: loadError || 'Function not loaded',
      }));
      return;
    }

    // Parse request body
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });

    req.on('end', async () => {
      const startTime = Date.now();
      const logs = [];

      try {
        // Parse event from request
        let event = {};
        if (body) {
          const parsed = JSON.parse(body);
          event = parsed.event || parsed;
        }

        console.log(`[Runtime] Invoking function with event:`, JSON.stringify(event));

        // Capture console output
        const originalLog = console.log;
        const originalError = console.error;
        const originalWarn = console.warn;

        console.log = (...args) => {
          const msg = args.map(a => String(a)).join(' ');
          logs.push(msg);
          originalLog('[Function]', ...args);
        };
        console.error = (...args) => {
          const msg = '[ERROR] ' + args.map(a => String(a)).join(' ');
          logs.push(msg);
          originalError('[Function]', ...args);
        };
        console.warn = (...args) => {
          const msg = '[WARN] ' + args.map(a => String(a)).join(' ');
          logs.push(msg);
          originalWarn('[Function]', ...args);
        };

        // Invoke the handler
        const result = await Promise.resolve(handler(event));

        // Restore console
        console.log = originalLog;
        console.error = originalError;
        console.warn = originalWarn;

        const duration = Date.now() - startTime;

        // Return success response
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          status: 'success',
          duration_ms: duration,
          response: result,
          logs: logs,
        }));

        console.log(`[Runtime] Function completed in ${duration}ms`);

      } catch (error) {
        const duration = Date.now() - startTime;

        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          status: 'error',
          duration_ms: duration,
          error: error.message,
          stack: error.stack,
          logs: logs,
        }));

        console.error(`[Runtime] Function failed: ${error.message}`);
      }
    });

    return;
  }

  // 404 for all other routes
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({
    error: 'Not found',
    endpoints: {
      'GET /health': 'Health check',
      'POST /invoke': 'Execute function',
      'POST /reload': 'Reload function code',
    },
  }));
});

// Start server
server.listen(PORT, '0.0.0.0', () => {
  console.log(`[Runtime] Node.js function runtime listening on port ${PORT}`);
  console.log(`[Runtime] Function handler: ${FUNCTION_HANDLER}`);
  console.log(`[Runtime] Code path: ${FUNCTION_CODE_PATH}`);

  // Try to load function code on startup
  loadFunction();
});

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('[Runtime] Received SIGTERM, shutting down gracefully...');
  server.close(() => {
    console.log('[Runtime] Server closed');
    process.exit(0);
  });
});

process.on('SIGINT', () => {
  console.log('[Runtime] Received SIGINT, shutting down gracefully...');
  server.close(() => {
    console.log('[Runtime] Server closed');
    process.exit(0);
  });
});
