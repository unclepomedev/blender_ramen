import bpy
import socket
import threading


class LiveLinkServer:
    def __init__(self, host="127.0.0.1", port=8080):
        self.host = host
        self.port = port
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.server_socket.bind((self.host, self.port))
        self.running = True

    def start(self):
        self.server_socket.listen(1)
        print(f"🍜 Blender Ramen: Listening on {self.host}:{self.port}...")

        while self.running:
            try:
                self.server_socket.settimeout(1.0)
                client, addr = self.server_socket.accept()

                data = b""
                while True:
                    packet = client.recv(4096)
                    if not packet:
                        break
                    data += packet

                script = data.decode("utf-8")
                client.close()

                if script:
                    print("✅ Received script from Rust, executing...")
                    bpy.app.timers.register(lambda: self.execute_script(script))

            except socket.timeout:
                continue
            except Exception as e:
                print(f"Error: {e}")

    def execute_script(self, script):
        try:
            exec(script, globals())
        except Exception as e:
            print(f"❌ Script execution failed:\n{e}")
        return None


if "ramen_server" in globals():
    globals()["ramen_server"].running = False

server = LiveLinkServer()
globals()["ramen_server"] = server

thread = threading.Thread(target=server.start)
thread.daemon = True
thread.start()
