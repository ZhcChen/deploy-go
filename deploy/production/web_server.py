#!/usr/bin/env python3
"""Deploy Go Web 生产静态服务：SPA 静态文件 + /api、/external 反向代理。"""

import argparse
import http.client
import mimetypes
import os
import posixpath
import selectors
import socket
import ssl
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
MAX_PROXY_BODY_BYTES = 2 * 1024 * 1024 * 1024
STREAM_CHUNK_BYTES = 64 * 1024


class RequestBodyError(Exception):
    pass


class ContentLengthBody:
    def __init__(self, source, length: int) -> None:
        self.source = source
        self.remaining = length

    def read(self, size: int = STREAM_CHUNK_BYTES) -> bytes:
        if self.remaining == 0:
            return b""
        chunk = self.source.read(min(size, self.remaining))
        if not chunk:
            raise RequestBodyError("request body ended before Content-Length")
        self.remaining -= len(chunk)
        return chunk


class ChunkedRequestBody:
    def __init__(self, source, limit: int = MAX_PROXY_BODY_BYTES) -> None:
        self.source = source
        self.limit = limit
        self.total = 0
        self.remaining = 0
        self.finished = False

    def read(self, size: int = STREAM_CHUNK_BYTES) -> bytes:
        if self.finished:
            return b""
        if self.remaining == 0:
            self._begin_chunk()
            if self.finished:
                return b""
        chunk = self.source.read(min(size, self.remaining))
        if not chunk:
            raise RequestBodyError("chunked request body ended unexpectedly")
        self.remaining -= len(chunk)
        self.total += len(chunk)
        if self.total > self.limit:
            raise RequestBodyError("request body exceeds proxy limit")
        if self.remaining == 0 and self.source.read(2) != b"\r\n":
            raise RequestBodyError("invalid chunk delimiter")
        return chunk

    def _begin_chunk(self) -> None:
        line = self.source.readline(130)
        if not line.endswith(b"\r\n") or len(line) > 128:
            raise RequestBodyError("invalid chunk header")
        try:
            self.remaining = int(line[:-2].split(b";", 1)[0], 16)
        except ValueError as error:
            raise RequestBodyError("invalid chunk size") from error
        if self.remaining != 0:
            return
        while True:
            trailer = self.source.readline(8194)
            if not trailer.endswith(b"\r\n") or len(trailer) > 8192:
                raise RequestBodyError("invalid chunk trailer")
            if trailer == b"\r\n":
                break
        self.finished = True


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
        if (
            path == "/api"
            or path.startswith("/api/")
            or path == "/external"
            or path.startswith("/external/")
        ):
            if self._is_websocket_upgrade():
                self._proxy_websocket()
            else:
                self._proxy_api()
            return
        self._serve_static()

    def _is_websocket_upgrade(self) -> bool:
        connection_tokens = {
            token.strip().lower()
            for token in self.headers.get("Connection", "").split(",")
        }
        return (
            self.command == "GET"
            and "upgrade" in connection_tokens
            and self.headers.get("Upgrade", "").lower() == "websocket"
        )

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

        try:
            body, encode_chunked = self._request_body()
        except RequestBodyError:
            self.send_error(400, "Invalid request body framing")
            return

        headers = {"Host": parsed.netloc}
        for key, value in self.headers.items():
            if key.lower() in HOP_BY_HOP or key.lower() == "host":
                continue
            headers[key] = value
        if encode_chunked:
            headers["Transfer-Encoding"] = "chunked"
        headers["X-Forwarded-For"] = self.client_address[0]
        headers["X-Forwarded-Proto"] = parsed.scheme

        connection = connection_type(host, port, timeout=None)
        try:
            connection.request(
                self.command,
                self.path,
                body=body,
                headers=headers,
                encode_chunked=encode_chunked,
            )
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
                # read1 返回当前已到达的数据，SSE 日志才能逐帧转发；
                # read() 会等待 64KiB 或上游结束，导致日志只在部署结束后出现。
                chunk = response.read1(65536)
                if not chunk:
                    break
                self.wfile.write(chunk)
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            self.close_connection = True
        except RequestBodyError:
            self.close_connection = True
            self.send_error(400, "Invalid request body framing")
        except OSError as error:
            self.send_error(502, str(error))
        finally:
            connection.close()

    def _request_body(self):
        transfer_encoding = self.headers.get("Transfer-Encoding")
        content_length = self.headers.get("Content-Length")
        if transfer_encoding and content_length:
            raise RequestBodyError("ambiguous request body framing")
        if transfer_encoding:
            if transfer_encoding.strip().lower() != "chunked":
                raise RequestBodyError("unsupported transfer encoding")
            return ChunkedRequestBody(self.rfile), True
        if content_length is None:
            return None, False
        try:
            length = int(content_length)
        except ValueError as error:
            raise RequestBodyError("invalid Content-Length") from error
        if length < 0 or length > MAX_PROXY_BODY_BYTES:
            raise RequestBodyError("request body exceeds proxy limit")
        return (ContentLengthBody(self.rfile, length) if length else None), False

    def _proxy_websocket(self) -> None:
        parsed = urllib.parse.urlsplit(self.api_base)
        host = parsed.hostname
        if not host:
            self.send_error(502)
            return
        port = parsed.port or (443 if parsed.scheme == "https" else 80)

        upstream = None
        response_started = False
        try:
            upstream = socket.create_connection((host, port), timeout=10)
            if parsed.scheme == "https":
                upstream = ssl.create_default_context().wrap_socket(
                    upstream, server_hostname=host
                )
            upstream.settimeout(None)

            headers = {"Host": parsed.netloc}
            for key, value in self.headers.items():
                if key.lower() in {"host", "proxy-connection"}:
                    continue
                headers[key] = value
            headers["X-Forwarded-For"] = self.client_address[0]
            headers["X-Forwarded-Proto"] = parsed.scheme
            request = [f"{self.command} {self.path} HTTP/1.1\r\n"]
            request.extend(f"{key}: {value}\r\n" for key, value in headers.items())
            request.append("\r\n")
            upstream.sendall("".join(request).encode("latin-1"))

            response_head = self._read_response_head(upstream)
            self.connection.sendall(response_head)
            response_started = True
            if not response_head.startswith((b"HTTP/1.1 101 ", b"HTTP/1.0 101 ")):
                return

            self.close_connection = True
            self._relay_websocket(upstream)
        except (BrokenPipeError, ConnectionResetError, TimeoutError, OSError):
            self.close_connection = True
            if not response_started:
                self.send_error(502)
        finally:
            if upstream is not None:
                upstream.close()

    @staticmethod
    def _read_response_head(upstream: socket.socket) -> bytes:
        response = bytearray()
        while b"\r\n\r\n" not in response:
            chunk = upstream.recv(4096)
            if not chunk:
                break
            response.extend(chunk)
            if len(response) > 65536:
                raise OSError("upstream response headers too large")
        return bytes(response)

    def _relay_websocket(self, upstream: socket.socket) -> None:
        selector = selectors.DefaultSelector()
        selector.register(self.connection, selectors.EVENT_READ, upstream)
        selector.register(upstream, selectors.EVENT_READ, self.connection)
        try:
            while True:
                for key, _ in selector.select():
                    data = key.fileobj.recv(65536)
                    if not data:
                        return
                    key.data.sendall(data)
        finally:
            selector.close()


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
