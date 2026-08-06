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


if __name__ == "__main__":
    unittest.main()
