#!/usr/bin/env python

from websocket import create_connection
import time

ws = create_connection(
    "ws://localhost:7575/client?client_id=00000000-0000-0000-0000-000000000000&room_id=0",
    timeout=1,
)

try:
    while True:
        time.sleep(0.01)
        ws.settimeout(1)
        ws.ping("")
        print("PING")

        try:
            received_data = ws.recv()
            print(received_data)
        except KeyboardInterrupt:
            break
        except:
            print("TIMEOUT RECEIVING")
except:
    print("EXITING")

ws.close()
