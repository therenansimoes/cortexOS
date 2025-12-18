import socket
import json
import time

PORT = 7077

def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # No Mac, para ouvir Broadcast, bind no 0.0.0.0 ou '' é suficiente
    try:
        sock.bind(('', PORT))
    except OSError as e:
        print(f"❌ Erro ao fazer bind na porta {PORT}: {e}")
        print("Verifique se outro listener não está rodando (use 'lsof -i :{PORT}')")
        return

    print(f"📡 Escutando por Broadcasts do CortexOS na porta {PORT}...")
    print(f"   (Certifique-se que o iPhone e o Mac estão no MESMO Wi-Fi)")

    while True:
        try:
            data, addr = sock.recvfrom(1024)
            try:
                msg = json.loads(data.decode('utf-8'))
                print(f"\n✨ RECEBIDO de {addr[0]}:")
                print(json.dumps(msg, indent=2))
            except json.JSONDecodeError:
                print(f"\n⚠️ Dados brutos de {addr[0]}: {data}")
        except KeyboardInterrupt:
            print("\nParando...")
            break

if __name__ == '__main__':
    main()
