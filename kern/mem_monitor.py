import serial
import sys
import time


ETH_BASE = 0xff3f0000
GMAC_MII_ADDR = 0x00000010
GMAC_MII_DATA = 0x00000014
MII_BUSY = 1
MII_WRITE = 2


class Monitor(object):
    def __init__(self, ser):
        self.ser = ser

    def read(self, size):
        res = self.ser.read(size)
        try:
            sys.stdout.write(res.decode('utf8', 'ignore'))
        except Exception:
            pass
        return res

    def wait_for_idle(self):
        result = b''
        num_empty = 0
        while num_empty < 3:
            res = self.read(1000)
            result += res
            if len(res) == 0:
                num_empty += 1
            else:
                num_empty = 0
        return result

    def read_word(self, addr):
        self.ser.write('mem read u32 {}\n'.format(addr).encode('ascii'))
        while True:
            line = self.ser.readline().strip().decode('ascii', 'ignore')
            if line.startswith('read:'):
                return eval(line[6:])

    def write_word(self, addr, data):
        self.ser.write('mem write u32 {} {}\n'.format(addr, data).encode('ascii'))
        while True:
            line = self.ser.readline().strip().decode('ascii', 'ignore')
            if line.startswith('write:'):
                return

    def mdio_wait_not_busy(self):
        while (self.read_word(ETH_BASE + GMAC_MII_ADDR) & MII_BUSY) != 0:
            time.sleep(0.05)

    def mdio_read(self, addr, reg):
        value = MII_BUSY
        value |= (addr << 11)
        value |= (reg << 6)

        self.mdio_wait_not_busy()

        self.write_word(ETH_BASE + GMAC_MII_DATA, 0)
        self.write_word(ETH_BASE + GMAC_MII_ADDR, value)

        self.mdio_wait_not_busy()

        return self.read_word(ETH_BASE + GMAC_MII_DATA) & 0xffff

    def mdio_write(self, addr, reg, data):
        value = MII_BUSY | MII_WRITE
        value |= (addr << 11)
        value |= (reg << 6)

        self.mdio_wait_not_busy()

        self.write_word(ETH_BASE + GMAC_MII_DATA, data)
        self.write_word(ETH_BASE + GMAC_MII_ADDR, value)

        self.mdio_wait_not_busy()

    def loop(self):
        self.ser.write(b'\n')
        if b'1/0$\r\n' not in self.read(100):
            print('shell working')
        else:
            print('waiting for boot+dwmac complete')
            while True:
                result = self.wait_for_idle()
                if b'[INFO:dwmac]' in result:
                    break
            print('mon: dwmac init complete')

        # for _ in range(1):
        #     for i in range(16):
        #         print('reg {}: {}'.format(i, [self.mdio_read(p, i) for p in range(2)]))
            # time.sleep(5)

        f = self.read_word(ETH_BASE + 0xd8)
        print(hex(f))
        # f |= (1 << 12) | (1 << 9)
        # print(hex(f))
        # self.write_word(ETH_BASE + 0xc0, f)


def main():
    with serial.Serial('/dev/ttyUSB0', 115200, timeout=0.2) as ser:
        Monitor(ser).loop()


if __name__ == '__main__':
    main()
