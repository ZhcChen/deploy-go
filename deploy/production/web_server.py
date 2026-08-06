#!/usr/bin/env python3
"""Deploy Go Web 生产静态服务：SPA 静态文件 + /api 反向代理。"""

import argparse
import http.client
import mimetypes
import os
import posixpath
import sys
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


HOP_BY_HOP = {
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
}


class DeployGoWebHandler(BaseHTTPRequestHandler):
    server_version = "DeployGoWeb/1.0"
    protocol_version = "HTTP/1.1"
    web_root = ""
    api_base = ""

    def do_HEAD(self) -> None:  # noqa: N802
        self._dispatch()

    def do_GET(self) -> None:  # noqa: N802
        self._dispatch()

    def do_POST(self) -> None:  # noqa: N802
        self._dispatch()

    def do_PUT(self) -> None:  # noqa: N802
        self._dispatch()

    def do_PATCH(self) -> None:  # noqa: N802
        self._dispatch()

    def do_DELETE(self) -> None:  # noqa: N802
        self._dispatch()

    def do_OPTIONS(self) -> None:  # noqa: N802
        self._dispatch()

    def _dispatch(self) -> None:
        path = urllib.parse.urlsplit(self.path).path
        if path == "/api" or path.startswith("/api/"):
            self._proxy_api()
            return
        self._serve_static()

    def _serve_static(self) -> None:
        path = urllib.parse.urlsplit(self.path).path
        relative = posixpath.normpath(urllib.parse.unquote(path)).lstrip("/")
        if relative.startswith("..") or ".." in relative.split("/"):
            self.send_error(404)
            return

        full_path = os.path.join(self.web_root, relative) if relative else self.web_root
        if os.path.isdir(full_path):
            full_path = os.path.join(full_path, "index.html")

        if not os.path.isfile(full_path):
            if self._looks_like_asset(path):
                self.send_error(404)
                return
            if self.command not in {"GET", "HEAD"}:
                self.send_error(405)
                return
            full_path = os.path.join(self.web_root, "index.html")

        if not os.path.isfile(full_path):
            self.send_error(404)
            return
        self._send_file(full_path, relative)

    def _looks_like_asset(self, path: str) -> bool:
        if path.startswith("/assets/"):
            return True
        last_segment = posixpath.basename(path)
        return bool(os.path.splitext(last_segment)[1])

    def _send_file(self, full_path: str, relative: str) -> None:
        try:
            size = os.path.getsize(full_path)
        except OSError:
            self.send_error(404)
            return

        cache_control = "no-cache"
        if relative.startswith("assets/"):
            cache_control = "public, max-age=31536000, immutable"
        elif relative in {"logo.svg", "favicon.ico"}:
            cache_control = "public, max-age=86400"

        content_type = mimetypes.guess_type(full_path)[0] or "application/octet-stream"
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(size))
        self.send_header("Cache-Control", cache_control)
        self.end_headers()
        if self.command == "HEAD":
            return

        try:
            with open(full_path, "rb") as handle:
                while True:
                    chunk = handle.read(65536)
                    if not chunk:
                        break
                    self.wfile.write(chunk)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            self.close_connection = True

    def _proxy_api(self) -> None:
        parsed = urllib.parse.urlsplit(self.api_base)
        host = parsed.hostname
        if not host:
            self.send_error(502)
            return
        port = parsed.port or (443 if parsed.scheme == "https" else 80)
        connection_type = (
            http.client.HTTPSConnection
            if parsed.scheme == "https"
            else http.client.HTTPConnection
        )

        body = None
        if self.command in {"POST", "PUT", "PATCH", "DELETE"}:
            try:
                length = int(self.headers.get("Content-Length") or "0")
            except ValueError:
                length = 0
            if length > 0:
                body = self.rfile.read(length)

        headers = {"Host": parsed.netloc}
        for key, value in self.headers.items():
            if key.lower() in HOP_BY_HOP or key.lower() == "host":
                continue
            headers[key] = value
        headers["X-Forwarded-For"] = self.client_address[0]
        headers["X-Forwarded-Proto"] = parsed.scheme

        connection = connection_type(host, port, timeout=None)
        try:
            connection.request(self.command, self.path, body=body, headers=headers)
            response = connection.getresponse()
            self.send_response_only(response.status, response.reason)
            for key, value in response.getheaders():
                if key.lower() in HOP_BY_HOP:
                    continue
                self.send_header(key, value)
            self.send_header("Connection", "close")
            self.end_headers()
            self.close_connection = True

            if self.command == "HEAD":
                response.close()
                return

            while True:
                chunk = response.read(65536)
                if not chunk:
                    break
                self.wfile.write(chunk)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            self.close_connection = True
        except OSError as error:
            self.send_error(502, str(error))
        finally:
            connection.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Deploy Go Web 静态服务")
    parser.add_argument("--root", default="/opt/deploy-go/web", help="Web 静态文件目录")
    parser.add_argument("--api", default="http://127.0.0.1:30100", help="API 基础地址")
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--port", type=int, default=30101)
    args = parser.parse_args()

    if not os.path.isdir(args.root):
        print(f"Web 根目录不存在：{args.root}", file=sys.stderr)
        raise SystemExit(1)

    DeployGoWebHandler.web_root = os.path.abspath(args.root)
    DeployGoWebHandler.api_base = args.api.rstrip("/")
    server = ThreadingHTTPServer((args.bind, args.port), DeployGoWebHandler)
    print(f"Deploy Go Web listening on http://{args.bind}:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
