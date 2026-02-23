import queue
import socket
import threading
import traceback

import bpy

MAX_SCRIPT_SIZE = 10 * 1024 * 1024  # 10 MB
LIVE_LINK_PORT = 8080


class LiveLinkServer:
    def __init__(self, host="127.0.0.1", port=LIVE_LINK_PORT):
        self.host = host
        self.port = port
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            self.server_socket.bind((self.host, self.port))
        except OSError:
            self.server_socket.close()
            raise
        self.running = True

    def start(self):
        self.server_socket.listen(1)
        print(f"🍜 Blender Ramen: Listening on {self.host}:{self.port}...")
        self.server_socket.settimeout(1.0)

        while self.running:
            try:
                client, _addr = self.server_socket.accept()
                client.settimeout(5.0)

                try:
                    data = b""
                    while True:
                        packet = client.recv(4096)
                        if not packet:
                            break
                        data += packet
                        if len(data) > MAX_SCRIPT_SIZE:
                            print(
                                "❌ Received data exceeds maximum allowed size, dropping."
                            )
                            data = b""
                            break

                    script = data.decode("utf-8")

                    if script:
                        print("✅ Received script from Rust, executing...")

                        res_q = queue.Queue()

                        def task(s=script, q=res_q):
                            try:
                                exec(s, globals())
                                q.put(b"OK")
                            except Exception:
                                q.put(
                                    f"ERROR\n{traceback.format_exc()}".encode("utf-8")
                                )
                            return None

                        bpy.app.timers.register(task)

                        try:
                            response = res_q.get(timeout=5.0)
                        except queue.Empty:
                            response = b"ERROR\nExecution timed out in Blender."
                        client.sendall(response)

                finally:
                    client.close()

            except socket.timeout:
                continue
            except (OSError, UnicodeDecodeError) as e:
                if self.running:
                    print(f"❌ Server error: {e}")

    def stop(self):
        self.running = False
        self.server_socket.close()


if "ramen_server" in globals():
    globals()["ramen_server"].stop()

try:
    server = LiveLinkServer()
    globals()["ramen_server"] = server

    thread = threading.Thread(target=server.start)
    thread.daemon = True
    thread.start()
except OSError as err:
    print(
        f"❌ Blender Ramen: Failed to start live-link server on port {LIVE_LINK_PORT}: {err}"
    )
