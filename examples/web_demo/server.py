#!/usr/bin/env python3
import http.server
import os

class CORSHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
        '.json': 'application/json',
        '.html': 'text/html',
        '.css': 'text/css',
        '.png': 'image/png',
        '.svg': 'image/svg+xml',
    }

    def end_headers(self):
        self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        super().end_headers()

if __name__ == '__main__':
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'dist'))
    print(f"Serving from: {os.getcwd()}")
    print(f"Listening on: http://localhost:7799")
    http.server.ThreadingHTTPServer(('0.0.0.0', 7799), CORSHandler).serve_forever()
