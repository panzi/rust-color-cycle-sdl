#!/usr/bin/env python3

import os
import sys
import termios

ttyinfo = termios.tcgetattr(0)
ttyinfo[3] &= ~(termios.ICANON | termios.ECHO)
termios.tcsetattr(0, termios.TCSANOW, ttyinfo)

try:
    fp = os.fdopen(0, "rb", buffering=0)
    while True:
        buf = fp.read()
        if buf:
            print(repr(buf))
except KeyboardInterrupt:
    print("^C")

finally:
    ttyinfo[3] |= termios.ICANON | termios.ECHO
    termios.tcsetattr(0, termios.TCSANOW, ttyinfo)
