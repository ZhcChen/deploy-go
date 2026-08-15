#!/usr/bin/env python3

import socket
import socketserver
import tempfile
import threading
import unittest
import urllib.request
from http.server import ThreadingHTTPServer

from web_server import DeployGoWebHandler


def read_headers(connection: socket.socket) -> bytes:
    data = bytearray()
    while b"\r\n\r\n" not in data:
        chunk = connection.recv(4096)
        if not chunk:
            break
        data.extend(chunk)
    return bytes(data)


class WebSocketUpstream(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        request = read_headers(self.request)
        self.server.request_headers = request
        self.request.sendall(
            b"HTTP/1.1 101 Switching Protocols\r\n"
            b"Connection: Upgrade\r\n"
            b"Upgrade: websocket\r\n\r\n"
        )
        payload = self.request.recv(4)
        self.request.sendall(payload)


class HttpUpstream(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        self.server.request_headers = read_headers(self.request)
        body = b'{"status":"ready"}'
        self.request.sendall(
            b"HTTP/1.1 200 OK\r\n"
            + f"Content-Length: {len(body)}\r\n".encode()
            + b"Content-Type: application/json\r\n\r\n"
            + body
        )


class UploadUpstream(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        self._stream = self.request.makefile("rb")
        header_lines = []
        while True:
            line = self._stream.readline()
            header_lines.append(line)
            if line == b"\r\n":
                break
        headers = b"".join(header_lines)
        self.server.request_headers = headers
        header_text = headers.decode("latin-1")
        content_length = 0
        for line in header_text.split("\r\n"):
            if line.lower().startswith("content-length:"):
                content_length = int(line.split(":", 1)[1].strip())
        body = bytearray()
        if "transfer-encoding: chunked" in header_text.lower():
            while True:
                size = int(self._readline().split(b";", 1)[0], 16)
                if size == 0:
                    self._readline()
                    break
                body.extend(self._read_exact(size))
                self._read_exact(2)
        else:
            body.extend(self._read_exact(content_length))
        self.server.request_body = bytes(body)
        response = b'{"stored":true}'
        self.request.sendall(
            b"HTTP/1.1 200 OK\r\n"
            + f"Content-Length: {len(response)}\r\n".encode()
            + b"Content-Type: application/json\r\n\r\n"
            + response
        )

    def _readline(self) -> bytes:
        return self._stream.readline()[:-2]

    def _read_exact(self, length: int) -> bytes:
        return self._stream.read(length)


class SseUpstream(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        self.server.request_headers = read_headers(self.request)
        self.request.sendall(
            b"HTTP/1.1 200 OK\r\n"
            b"Content-Type: text/event-stream\r\n\r\n"
            b"id: 1\nevent: log\ndata: {\"sequence\":1}\n\n"
        )
        self.server.first_event_received.wait(5)
        self.request.sendall(b"event: terminal\ndata: {}\n\n")


class QuietDeployGoWebHandler(DeployGoWebHandler):
    def log_message(self, _format: str, *_args: object) -> None:
        pass


class WebServerContractTest(unittest.TestCase):
    def test_websocket_upgrade_and_payload_are_forwarded(self) -> None:
        upstream = socketserver.ThreadingTCPServer(("127.0.0.1", 0), WebSocketUpstream)
        upstream.request_headers = b""
        upstream_thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        upstream_thread.start()

        with tempfile.TemporaryDirectory() as web_root:
            QuietDeployGoWebHandler.web_root = web_root
            QuietDeployGoWebHandler.api_base = (
                f"http://127.0.0.1:{upstream.server_address[1]}"
            )
            proxy = ThreadingHTTPServer(("127.0.0.1", 0), QuietDeployGoWebHandler)
            proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            proxy_thread.start()
            try:
                with socket.create_connection(proxy.server_address, timeout=3) as client:
                    client.sendall(
                        b"GET /api/v1/agent/control HTTP/1.1\r\n"
                        b"Host: deploy.example.test\r\n"
                        b"Authorization: Bearer fixture-token\r\n"
                        b"Connection: keep-alive, Upgrade\r\n"
                        b"Upgrade: websocket\r\n"
                        b"Sec-WebSocket-Version: 13\r\n"
                        b"Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
                    )
                    response = read_headers(client)
                    self.assertTrue(response.startswith(b"HTTP/1.1 101 "), response)
                    client.sendall(b"ping")
                    self.assertEqual(client.recv(4), b"ping")

                self.assertIn(b"Connection: keep-alive, Upgrade", upstream.request_headers)
                self.assertIn(b"Upgrade: websocket", upstream.request_headers)
                self.assertIn(b"Authorization: Bearer fixture-token", upstream.request_headers)
            finally:
                proxy.shutdown()
                proxy.server_close()
                upstream.shutdown()
                upstream.server_close()

    def test_regular_api_requests_still_use_http_proxy(self) -> None:
        upstream = socketserver.ThreadingTCPServer(("127.0.0.1", 0), HttpUpstream)
        upstream.request_headers = b""
        upstream_thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        upstream_thread.start()

        with tempfile.TemporaryDirectory() as web_root:
            QuietDeployGoWebHandler.web_root = web_root
            QuietDeployGoWebHandler.api_base = (
                f"http://127.0.0.1:{upstream.server_address[1]}"
            )
            proxy = ThreadingHTTPServer(("127.0.0.1", 0), QuietDeployGoWebHandler)
            proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            proxy_thread.start()
            try:
                url = f"http://127.0.0.1:{proxy.server_address[1]}/api/readyz"
                with urllib.request.urlopen(url, timeout=3) as response:
                    self.assertEqual(response.read(), b'{"status":"ready"}')
                self.assertIn(b"GET /api/readyz HTTP/1.1", upstream.request_headers)
                self.assertNotIn(b"Upgrade: websocket", upstream.request_headers)
            finally:
                proxy.shutdown()
                proxy.server_close()
                upstream.shutdown()
                upstream.server_close()

    def test_sse_response_is_flushed_before_upstream_terminates(self) -> None:
        upstream = socketserver.ThreadingTCPServer(("127.0.0.1", 0), SseUpstream)
        upstream.request_headers = b""
        upstream.first_event_received = threading.Event()
        upstream_thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        upstream_thread.start()

        with tempfile.TemporaryDirectory() as web_root:
            QuietDeployGoWebHandler.web_root = web_root
            QuietDeployGoWebHandler.api_base = (
                f"http://127.0.0.1:{upstream.server_address[1]}"
            )
            proxy = ThreadingHTTPServer(("127.0.0.1", 0), QuietDeployGoWebHandler)
            proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            proxy_thread.start()
            try:
                with socket.create_connection(proxy.server_address, timeout=3) as client:
                    client.sendall(
                        b"GET /api/v1/deployments/deployment-1/logs HTTP/1.1\r\n"
                        b"Host: deploy.example.test\r\n"
                        b"Accept: text/event-stream\r\n\r\n"
                    )
                    response = read_headers(client)
                    self.assertTrue(response.startswith(b"HTTP/1.1 200 "), response)
                    client.settimeout(2)
                    _, _, first = response.partition(b"\r\n\r\n")
                    if not first:
                        first = client.recv(4096)
                    self.assertIn(b"event: log", first)
                    upstream.first_event_received.set()
                    terminal = client.recv(4096)
                    self.assertIn(b"event: terminal", terminal)
            finally:
                proxy.shutdown()
                proxy.server_close()
                upstream.shutdown()
                upstream.server_close()

    def test_external_requests_are_proxied_to_api(self) -> None:
        upstream = socketserver.ThreadingTCPServer(("127.0.0.1", 0), HttpUpstream)
        upstream.request_headers = b""
        upstream_thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        upstream_thread.start()

        with tempfile.TemporaryDirectory() as web_root:
            QuietDeployGoWebHandler.web_root = web_root
            QuietDeployGoWebHandler.api_base = (
                f"http://127.0.0.1:{upstream.server_address[1]}"
            )
            proxy = ThreadingHTTPServer(("127.0.0.1", 0), QuietDeployGoWebHandler)
            proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            proxy_thread.start()
            try:
                url = (
                    f"http://127.0.0.1:{proxy.server_address[1]}"
                    "/external/v1/openapi.json"
                )
                with urllib.request.urlopen(url, timeout=3) as response:
                    self.assertEqual(response.read(), b'{"status":"ready"}')
                self.assertIn(
                    b"GET /external/v1/openapi.json HTTP/1.1",
                    upstream.request_headers,
                )
            finally:
                proxy.shutdown()
                proxy.server_close()
                upstream.shutdown()
                upstream.server_close()

    def test_content_length_and_chunked_uploads_are_streamed(self) -> None:
        upstream = socketserver.ThreadingTCPServer(("127.0.0.1", 0), UploadUpstream)
        upstream.request_headers = b""
        upstream.request_body = b""
        upstream_thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        upstream_thread.start()

        with tempfile.TemporaryDirectory() as web_root:
            QuietDeployGoWebHandler.web_root = web_root
            QuietDeployGoWebHandler.api_base = (
                f"http://127.0.0.1:{upstream.server_address[1]}"
            )
            proxy = ThreadingHTTPServer(("127.0.0.1", 0), QuietDeployGoWebHandler)
            proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            proxy_thread.start()
            try:
                payload = b"a" * (2 * 1024 * 1024 + 17)
                request = urllib.request.Request(
                    f"http://127.0.0.1:{proxy.server_address[1]}/api/v1/artifacts/upload",
                    data=payload,
                    method="PUT",
                )
                with urllib.request.urlopen(request, timeout=5) as response:
                    self.assertEqual(response.status, 200)
                self.assertEqual(upstream.request_body, payload)

                with socket.create_connection(proxy.server_address, timeout=5) as client:
                    client.sendall(
                        b"PUT /api/v1/artifacts/upload HTTP/1.1\r\n"
                        b"Host: deploy.example.test\r\n"
                        b"Transfer-Encoding: chunked\r\n\r\n"
                        b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"
                    )
                    response = read_headers(client)
                    self.assertTrue(response.startswith(b"HTTP/1.1 200 "), response)
                self.assertEqual(upstream.request_body, b"hello world")
                self.assertIn(
                    b"Transfer-Encoding: chunked", upstream.request_headers
                )
            finally:
                proxy.shutdown()
                proxy.server_close()
                upstream.shutdown()
                upstream.server_close()


if __name__ == "__main__":
    unittest.main()
