use stm32f4xx_hal::pac;

/// Which USART + DMA combination to use.
pub enum UartPort {
    /// USART1 RX → DMA2 Stream 2, Channel 4
    Usart1,
    /// USART2 RX → DMA1 Stream 5, Channel 4
    Usart2,
}

/// Line-buffered UART receiver backed by circular DMA.
///
/// Handles DMA setup, IDLE interrupt processing, byte extraction
/// via volatile reads, and newline-delimited line assembly.
/// Sensor modules use this instead of touching DMA registers directly.
pub struct UartDma {
    last_pos: usize,
    line_buf: [u8; 64],
    line_pos: usize,
}

impl UartDma {
    pub fn new() -> Self {
        Self {
            last_pos: 0,
            line_buf: [0u8; 64],
            line_pos: 0,
        }
    }

    /// Configure the DMA stream in circular mode for the given USART peripheral.
    pub fn init_dma(port: &UartPort, rx_buf: &[u8]) {
        match port {
            UartPort::Usart1 => Self::init_usart1(rx_buf),
            UartPort::Usart2 => Self::init_usart2(rx_buf),
        }
    }

    /// Handle the USART IDLE interrupt. Extracts new bytes from the DMA
    /// circular buffer, assembles newline-delimited lines, and calls
    /// `on_line` with each complete line as a byte slice.
    pub fn on_idle(
        &mut self,
        port: &UartPort,
        rx_buf: &[u8],
        mut on_line: impl FnMut(&[u8]),
    ) {
        match port {
            UartPort::Usart1 => self.handle_idle_usart1(rx_buf, &mut on_line),
            UartPort::Usart2 => self.handle_idle_usart2(rx_buf, &mut on_line),
        }
    }

    // ---- USART1 + DMA2 Stream 2 ------------------------------------------------

    fn init_usart1(rx_buf: &[u8]) {
        let usart1 = unsafe { &*pac::USART1::ptr() };
        let dma2 = unsafe { &*pac::DMA2::ptr() };

        dma2.st[2].cr.modify(|_, w| w.en().clear_bit());
        while dma2.st[2].cr.read().en().bit_is_set() {}

        dma2.lifcr.write(|w| {
            w.ctcif2()
                .set_bit()
                .chtif2()
                .set_bit()
                .cteif2()
                .set_bit()
                .cdmeif2()
                .set_bit()
                .cfeif2()
                .set_bit()
        });

        dma2.st[2]
            .par
            .write(|w| unsafe { w.bits(&usart1.dr as *const _ as u32) });
        dma2.st[2]
            .m0ar
            .write(|w| unsafe { w.bits(rx_buf.as_ptr() as u32) });
        dma2.st[2]
            .ndtr
            .write(|w| unsafe { w.bits(rx_buf.len() as u32) });

        // CHSEL=4, DIR=P→M, CIRC, MINC, PL=High, EN
        dma2.st[2].cr.write(|w| unsafe {
            w.bits(
                (4 << 25)
                    | (0 << 6)
                    | (1 << 8)
                    | (1 << 10)
                    | (2 << 16)
                    | (1 << 0),
            )
        });

        usart1.cr3.modify(|_, w| w.dmar().set_bit());
        usart1.cr1.modify(|_, w| w.idleie().set_bit());
    }

    fn handle_idle_usart1(&mut self, rx_buf: &[u8], on_line: &mut impl FnMut(&[u8])) {
        let usart1 = unsafe { &*pac::USART1::ptr() };
        let sr = usart1.sr.read();

        if sr.idle().bit_is_set() {
            let _ = usart1.dr.read();
            let dma2 = unsafe { &*pac::DMA2::ptr() };
            let ndtr = (dma2.st[2].ndtr.read().bits() & 0xFFFF) as usize;
            let head = rx_buf.len() - ndtr;
            self.extract_lines(rx_buf, head, on_line);
        }

        if sr.ore().bit_is_set() {
            let _ = usart1.dr.read();
        }
    }

    // ---- USART2 + DMA1 Stream 5 ------------------------------------------------

    fn init_usart2(rx_buf: &[u8]) {
        let usart2 = unsafe { &*pac::USART2::ptr() };
        let dma1 = unsafe { &*pac::DMA1::ptr() };

        dma1.st[5].cr.modify(|_, w| w.en().clear_bit());
        while dma1.st[5].cr.read().en().bit_is_set() {}

        dma1.hifcr.write(|w| {
            w.ctcif5()
                .set_bit()
                .chtif5()
                .set_bit()
                .cteif5()
                .set_bit()
                .cdmeif5()
                .set_bit()
                .cfeif5()
                .set_bit()
        });

        dma1.st[5]
            .par
            .write(|w| unsafe { w.bits(&usart2.dr as *const _ as u32) });
        dma1.st[5]
            .m0ar
            .write(|w| unsafe { w.bits(rx_buf.as_ptr() as u32) });
        dma1.st[5]
            .ndtr
            .write(|w| unsafe { w.bits(rx_buf.len() as u32) });

        // CHSEL=4, DIR=P→M, CIRC, MINC, PL=High, EN
        dma1.st[5].cr.write(|w| unsafe {
            w.bits(
                (4 << 25)
                    | (0 << 6)
                    | (1 << 8)
                    | (1 << 10)
                    | (2 << 16)
                    | (1 << 0),
            )
        });

        usart2.cr3.modify(|_, w| w.dmar().set_bit());
        usart2.cr1.modify(|_, w| w.idleie().set_bit());
    }

    fn handle_idle_usart2(&mut self, rx_buf: &[u8], on_line: &mut impl FnMut(&[u8])) {
        let usart2 = unsafe { &*pac::USART2::ptr() };
        let sr = usart2.sr.read();

        if sr.idle().bit_is_set() {
            let _ = usart2.dr.read();
            let dma1 = unsafe { &*pac::DMA1::ptr() };
            let ndtr = (dma1.st[5].ndtr.read().bits() & 0xFFFF) as usize;
            let head = rx_buf.len() - ndtr;
            self.extract_lines(rx_buf, head, on_line);
        }

        if sr.ore().bit_is_set() {
            let _ = usart2.dr.read();
        }
    }

    // ---- shared line assembly ---------------------------------------------------

    fn extract_lines(
        &mut self,
        rx_buf: &[u8],
        head: usize,
        on_line: &mut impl FnMut(&[u8]),
    ) {
        let buf_size = rx_buf.len();
        let tail = self.last_pos;

        if head == tail {
            return;
        }

        let count = if head >= tail {
            head - tail
        } else {
            buf_size - tail + head
        };

        let buf_ptr = rx_buf.as_ptr();
        for i in 0..count {
            let idx = (tail + i) % buf_size;
            let byte = unsafe { core::ptr::read_volatile(buf_ptr.add(idx)) };

            if byte == b'\n' || byte == b'\r' {
                if self.line_pos > 0 {
                    on_line(&self.line_buf[..self.line_pos]);
                    self.line_pos = 0;
                }
            } else if self.line_pos < self.line_buf.len() {
                self.line_buf[self.line_pos] = byte;
                self.line_pos += 1;
            } else {
                // Line too long — discard
                self.line_pos = 0;
            }
        }
        self.last_pos = head;
    }
}
