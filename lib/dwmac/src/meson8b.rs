
pub const PRG_ETH0: usize = 0x0;

pub const PRG_ETH0_RGMII_MODE: u32 = 1 << 0;

pub const PRG_ETH0_EXT_PHY_MODE_MASK: u32 = GENMASK(2, 0);
pub const PRG_ETH0_EXT_RGMII_MODE: usize = 1;
pub const PRG_ETH0_EXT_RMII_MODE: usize = 4;

/* mux to choose between fclk_div2 (bit unset) and mpll2 (bit set) */
pub const PRG_ETH0_CLK_M250_SEL_SHIFT: usize = 4;
pub const PRG_ETH0_CLK_M250_SEL_MASK: u32 = GENMASK(4, 4);

/* TX clock delay in ns = "8ns / 4 * tx_dly_val" (where 8ns are exactly one
 * cycle of the 125MHz RGMII TX clock):
 * 0ns = 0x0, 2ns = 0x1, 4ns = 0x2, 6ns = 0x3
 */
pub const PRG_ETH0_TXDLY_MASK: u32 = GENMASK(6, 5);

/* divider for the result of m250_sel */
pub const PRG_ETH0_CLK_M250_DIV_SHIFT: usize = 7;
pub const PRG_ETH0_CLK_M250_DIV_WIDTH: usize = 3;

pub const PRG_ETH0_RGMII_TX_CLK_EN: u32 = 1 << 10;

pub const PRG_ETH0_INVERTED_RMII_CLK: u32 = 1 << 11;
pub const PRG_ETH0_TX_AND_PHY_REF_CLK: u32 = 1 << 12;

/* Bypass (= 0, the signal from the GPIO input directly connects to the
 * internal sampling) or enable (= 1) the internal logic for RXEN and RXD[3:0]
 * timing tuning.
 */
pub const PRG_ETH0_ADJ_ENABLE: u32 = 1 << 13;
/* Controls whether the RXEN and RXD[3:0] signals should be aligned with the
 * input RX rising/falling edge and sent to the Ethernet internals. This sets
 * the automatically delay and skew automatically (internally).
 */
pub const PRG_ETH0_ADJ_SETUP: u32 = 1 << 14;
/* An internal counter based on the "timing-adjustment" clock. The counter is
 * cleared on both, the falling and rising edge of the RX_CLK. This selects the
 * delay (= the counter value) when to start sampling RXEN and RXD[3:0].
 */
pub const PRG_ETH0_ADJ_DELAY: u32 = GENMASK(19, 15);
/* Adjusts the skew between each bit of RXEN and RXD[3:0]. If a signal has a
 * large input delay, the bit for that signal (RXEN = bit 0, RXD[3] = bit 1,
 * ...) can be configured to be 1 to compensate for a delay of about 1ns.
 */
pub const PRG_ETH0_ADJ_SKEW: u32 = GENMASK(24, 20);

const fn GENMASK(h: usize, l: usize) -> u32 {
    ((u32::max_value() - (1u32 << l) + 1) & (u32::max_value() >> (32 - 1 - h)))
}


pub fn init() {
    unsafe {
        // Set External phy

        let mdio_mux_base = 0xff600000usize + 0x4c000;
        const ETH_PHY_CNTL2: usize = 0x88;

        crate::write_u32(mdio_mux_base + ETH_PHY_CNTL2, 0);


        // Set RGMII mode

        let addr2 = 0xff634540usize;
        let addr = addr2 as *mut u32;
        info!("amlogic: {:#08x} {:#08x}", (addr2 as *mut u32).read_volatile(), ((addr2+4) as *mut u32).read_volatile());
        let mut value = addr.read_volatile();

        value &= !PRG_ETH0_EXT_PHY_MODE_MASK;
        value |= PRG_ETH0_RGMII_MODE;

        addr.write_volatile(value);
        value = addr.read_volatile();

        value &= !(PRG_ETH0_TXDLY_MASK |
            				PRG_ETH0_ADJ_ENABLE | PRG_ETH0_ADJ_SETUP |
            				PRG_ETH0_ADJ_DELAY | PRG_ETH0_ADJ_SKEW);

        // tx delay 2ns
        value |= (1 << 5);
        addr.write_volatile(value);

        value |= PRG_ETH0_TX_AND_PHY_REF_CLK;
        value |= PRG_ETH0_RGMII_TX_CLK_EN;

        addr.write_volatile(value);
        //addr.write_volatile(1 | (1 << 5) | (1 << 12));
        info!("amlogic: {:#08x} {:#08x}", (addr2 as *mut u32).read_volatile(), ((addr2+4) as *mut u32).read_volatile());
    }
}

