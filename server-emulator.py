#!/usr/bin/env python

from websocket import create_connection
import time

room_list_packet = [
    0xEF,  # 0
    0xBE,  # 1
    0xED,  # 2
    0xFE,  # 3
    0x5E,  # 4
    0x00,  # 5
    0x00,  # 6
    0x00,  # 7
    0x00,  # 8
    0x0F,  # 9
    0x00,  # 10
    0x00,  # 11
    0x80,  # 12
    0x0C,  # 13
    0x00,  # 14
    0x00,  # 15
    0x00,  # 16
    0x1F,  # 17
    0x05,  # 18
    0x00,  # 19
    0x01,  # 20
    0x00,  # 21
    0x00,  # 22
    0x01,  # 23
    0x03,  # 24
]
room_wave_packet = [
    0xEF,  # 0
    0xBE,  # 1
    0xED,  # 2
    0xFE,  # 3
    0x00,  # 4
    0x00,  # 5
    0x00,  # 6
    0x00,  # 7
    0x00,  # 8
    0x00,  # 9
    0x00,  # 10
    0x00,  # 11
    0x80,  # 12
    0x00,  # 13
    0x00,  # 14
    0x00,  # 15
    0x00,  # 16
    0xDA,  # 17
    0x02,  # 18
    0x00,  # 19
    0xAA,  # 20
    0xBB,  # 21
]

ws = create_connection(
    "ws://localhost:7575/server?client_id=00000000-0000-0000-0000-000000000000",
    timeout=1,
)
print("SENDING: Room List")
ws.send_binary(room_list_packet)
print("SENT")

try:
    while True:
        time.sleep(0.01)
        ws.settimeout(1)
        ws.send_binary(room_wave_packet)
        ws.ping("")
        print("PING")
except:
    print("EXITING")

ws.close()
