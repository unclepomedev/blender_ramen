import bpy
import socket
import threading

MAX_SCRIPT_SIZE = 10 * 1024 * 1024  # 10 MB
LIVE_LINK_PORT = 8080


class LiveLinkServer:
    def __init__(self, host="127.0.0.1", port=LIVE_LINK_PORT):
        self.host = host
        self.port = port
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.server_socket.bind((self.host, self.port))
        self.running = True

    def start(self):
        self.server_socket.listen(1)
        print(f"🍜 Blender Ramen: Listening on {self.host}:{self.port}...")
        self.server_socket.settimeout(1.0)

        while self.running:
            try:
                client, _addr = self.server_socket.accept()

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
                finally:
                    client.close()

                if script:
                    print("✅ Received script from Rust, executing...")
                    bpy.app.timers.register(lambda s=script: self.execute_script(s))

            except socket.timeout:
                continue
            except (OSError, UnicodeDecodeError) as e:
                print(f"❌ Server error: {e}")

    @staticmethod
    def execute_script(script):
        try:
            # Note: Arbitrary code execution from localhost is by design. This tool assumes a trusted local development environment.
            exec(script, globals())
        except Exception as e:
            print(f"❌ Script execution failed:\n{e}")
        return None

    def stop(self):
        self.running = False
        self.server_socket.close()


if "ramen_server" in globals():
    globals()["ramen_server"].stop()

server = LiveLinkServer()
globals()["ramen_server"] = server

thread = threading.Thread(target=server.start)
thread.daemon = True
thread.start()
