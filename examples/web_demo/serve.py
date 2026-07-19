#!/usr/bin/env python3
import gzip
import http.server
import os
import sys

class GzipHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
        '.json': 'application/json',
    }

    def end_headers(self):
        self.send_header('Cache-Control', 'no-cache')
        super().end_headers()

    def do_GET(self):
        path = self.translate_path(self.path)
        if not os.path.isfile(path) or 'gzip' not in self.headers.get('Accept-Encoding', ''):
            return super().do_GET()

        with open(path, 'rb') as f:
            data = f.read()
        body = gzip.compress(data, 6)

        self.send_response(200, None)
        self.send_header('Content-Type', self.guess_type(path)[0])
        self.send_header('Content-Encoding', 'gzip')
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-cache')
        self.end_headers()
        self.wfile.write(body)

if __name__ == '__main__':
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'dist'))
    http.server.ThreadingHTTPServer(('0.0.0.0', 7799), GzipHandler).serve_forever()
