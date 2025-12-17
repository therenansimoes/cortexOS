import socket
import struct
import json

MCAST_GRP = '239.255.70.77'
MCAST_PORT = 7077

def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # Bind to all interfaces on port 7077
    sock.bind(('', MCAST_PORT))
    
    # Join multicast group
    mreq = struct.pack("4sl", socket.inet_aton(MCAST_GRP), socket.INADDR_ANY)
    sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

    print(f"🎧 Listening for CortexOS discovery on {MCAST_GRP}:{MCAST_PORT}...")

    while True:
        try:
            data, addr = sock.recvfrom(1024)
            try:
                msg = json.loads(data.decode('utf-8'))
                print(f"✨ Discovered Node from {addr}:")
                print(json.dumps(msg, indent=2))
            except json.JSONDecodeError:
                print(f"Received raw data from {addr}: {data}")
        except KeyboardInterrupt:
            break

if __name__ == '__main__':
    main()
