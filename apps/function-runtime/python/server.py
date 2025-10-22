#!/usr/bin/env python3

"""
Python Function Runtime Server

This HTTP server runs inside a Firecracker MicroVM and executes
serverless functions on demand. Each function gets its own VM.

Endpoints:
  GET  /health       - Health check
  POST /invoke       - Execute the function
  POST /reload       - Hot-reload function code
"""

import http.server
import json
import os
import sys
import time
import traceback
from io import StringIO
import importlib.util

PORT = int(os.environ.get('PORT', 3000))
FUNCTION_CODE_PATH = os.environ.get('FUNCTION_CODE_PATH', '/function/code.py')
FUNCTION_HANDLER = os.environ.get('FUNCTION_HANDLER', 'handler')

handler_func = None
load_error = None


def load_function():
    """Load (or reload) the function code"""
    global handler_func, load_error

    try:
        if not os.path.exists(FUNCTION_CODE_PATH):
            load_error = f"Function code not found at {FUNCTION_CODE_PATH}"
            print(f"[Runtime] {load_error}", file=sys.stderr)
            return False

        # Load the module
        spec = importlib.util.spec_from_file_location("user_function", FUNCTION_CODE_PATH)
        module = importlib.util.module_from_spec(spec)

        # Remove from sys.modules to enable hot-reloading
        if 'user_function' in sys.modules:
            del sys.modules['user_function']

        spec.loader.exec_module(module)

        # Get the handler function
        if not hasattr(module, FUNCTION_HANDLER):
            raise AttributeError(f"Handler '{FUNCTION_HANDLER}' not found in module")

        handler_func = getattr(module, FUNCTION_HANDLER)

        if not callable(handler_func):
            raise TypeError(f"Handler '{FUNCTION_HANDLER}' is not callable")

        load_error = None
        print(f"[Runtime] Loaded function handler: {FUNCTION_HANDLER}")
        return True

    except Exception as e:
        load_error = str(e)
        print(f"[Runtime] Failed to load function: {e}", file=sys.stderr)
        traceback.print_exc()
        return False


class FunctionRuntimeHandler(http.server.BaseHTTPRequestHandler):
    """HTTP request handler for function runtime"""

    def log_message(self, format, *args):
        """Override to customize logging"""
        print(f"[Runtime] {format % args}")

    def send_json_response(self, status_code, data):
        """Send JSON response"""
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode('utf-8'))

    def do_GET(self):
        """Handle GET requests"""
        if self.path == '/health':
            self.send_json_response(200, {
                'status': 'healthy',
                'handler': FUNCTION_HANDLER,
                'codeLoaded': handler_func is not None,
                'error': load_error,
            })
        else:
            self.send_json_response(404, {
                'error': 'Not found',
                'endpoints': {
                    'GET /health': 'Health check',
                    'POST /invoke': 'Execute function',
                    'POST /reload': 'Reload function code',
                },
            })

    def do_POST(self):
        """Handle POST requests"""
        if self.path == '/write-code':
            # Write new code to disk and reload
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else '{}'

            try:
                data = json.loads(body)
                code = data.get('code')

                if not code:
                    self.send_json_response(400, {'error': 'Missing code field'})
                    return

                # Write code to file
                with open(FUNCTION_CODE_PATH, 'w') as f:
                    f.write(code)

                # Reload the function
                success = load_function()

                self.send_json_response(200 if success else 500, {
                    'success': success,
                    'error': load_error,
                })
            except Exception as e:
                self.send_json_response(500, {'error': str(e)})

        elif self.path == '/reload':
            success = load_function()
            self.send_json_response(200 if success else 500, {
                'success': success,
                'error': load_error,
            })

        elif self.path == '/invoke':
            # Check if function is loaded
            if handler_func is None:
                self.send_json_response(500, {
                    'status': 'error',
                    'error': load_error or 'Function not loaded',
                })
                return

            # Parse request body
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else '{}'

            start_time = time.time()
            logs = []

            try:
                # Parse event from request
                parsed = json.loads(body)
                event = parsed.get('event', parsed)

                print(f"[Runtime] Invoking function with event: {json.dumps(event)}")

                # Capture stdout and stderr
                old_stdout = sys.stdout
                old_stderr = sys.stderr
                sys.stdout = StringIO()
                sys.stderr = StringIO()

                try:
                    # Invoke the handler
                    result = handler_func(event)

                    # Capture logs
                    stdout_value = sys.stdout.getvalue()
                    stderr_value = sys.stderr.getvalue()

                    if stdout_value:
                        logs.extend(stdout_value.strip().split('\n'))
                    if stderr_value:
                        logs.extend(['[ERROR] ' + line for line in stderr_value.strip().split('\n')])

                finally:
                    # Restore stdout/stderr
                    sys.stdout = old_stdout
                    sys.stderr = old_stderr

                duration_ms = int((time.time() - start_time) * 1000)

                # Send success response
                self.send_json_response(200, {
                    'status': 'success',
                    'duration_ms': duration_ms,
                    'response': result,
                    'logs': logs,
                })

                print(f"[Runtime] Function completed in {duration_ms}ms")

            except Exception as e:
                duration_ms = int((time.time() - start_time) * 1000)

                # Send error response
                self.send_json_response(500, {
                    'status': 'error',
                    'duration_ms': duration_ms,
                    'error': str(e),
                    'stack': traceback.format_exc(),
                    'logs': logs,
                })

                print(f"[Runtime] Function failed: {e}", file=sys.stderr)

        else:
            self.send_json_response(404, {
                'error': 'Not found',
                'endpoints': {
                    'GET /health': 'Health check',
                    'POST /invoke': 'Execute function',
                    'POST /reload': 'Reload function code',
                },
            })


def run_server():
    """Start the HTTP server"""
    server_address = ('0.0.0.0', PORT)
    httpd = http.server.HTTPServer(server_address, FunctionRuntimeHandler)

    print(f"[Runtime] Python function runtime listening on port {PORT}")
    print(f"[Runtime] Function handler: {FUNCTION_HANDLER}")
    print(f"[Runtime] Code path: {FUNCTION_CODE_PATH}")

    # Try to load function code on startup
    load_function()

    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n[Runtime] Received interrupt, shutting down gracefully...")
        httpd.server_close()
        print("[Runtime] Server closed")


if __name__ == '__main__':
    run_server()
