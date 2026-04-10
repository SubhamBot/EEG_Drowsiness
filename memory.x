/* memory.x for STM32F429ZI */
MEMORY
{
  /* 2 Megabytes of Flash Memory */
  FLASH : ORIGIN = 0x08000000, LENGTH = 2048K
  
  /* 192 Kilobytes of Continuous Standard SRAM */
  RAM   : ORIGIN = 0x20000000, LENGTH = 192K
}