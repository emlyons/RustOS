#define GPIO_BASE (0x3F000000 + 0x200000)

volatile unsigned *GPIO_FSEL1 = (volatile unsigned *)(GPIO_BASE + 0x04);
volatile unsigned *GPIO_SET0  = (volatile unsigned *)(GPIO_BASE + 0x1C);
volatile unsigned *GPIO_CLR0  = (volatile unsigned *)(GPIO_BASE + 0x28);

static void spin_sleep_us (unsigned int us) {
  for (unsigned int i = 0; i < us * 6; i++) {
    asm volatile("nop");
  }
}

static void spin_sleep_ms (unsigned int ms) {
  spin_sleep_us (ms * 1000);
}

int kmain (void) {

    // Set GPIO Pin 16 as output.
    const int pin = 16;
    int shift = (pin % 10) * 3;
    
    *GPIO_FSEL1 &= ~(0b111 << shift);
    *GPIO_FSEL1 |= (0b001 << shift);

    // Continuously set and clear GPIO 16.
    while (1) {

      *GPIO_SET0 &= ~(0b1 << pin);
      *GPIO_SET0 |= (0b1 << pin);
	
      spin_sleep_ms (200);
      
      *GPIO_CLR0 &= ~(0b1 << pin);
      *GPIO_CLR0 |= (0b1 << pin);
      
      spin_sleep_ms (200);
    }
    return 0;
}
