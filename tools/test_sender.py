import socket
import json

MCAST_GRP = '239.255.70.77'
MCAST_PORT = 7077

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 2)

msg = json.dumps({
    "cortex": True,
    "node_id": "TEST_SENDER",
    "type": "discovery",
    "agents": 1
})

print(f"Sending to {MCAST_GRP}:{MCAST_PORT}...")
sock.sendto(msg.encode('utf-8'), (MCAST_GRP, MCAST_PORT))
print("Sent!")
