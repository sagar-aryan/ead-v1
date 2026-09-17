#pragma once
// USB link: the doc-08 messages framed as 00 | COBS(message || CRC32) | 00
// (DEC-005), written directly to the USB Serial/JTAG endpoint FIFO from a core-0
// task. Never blocks acquisition: when the host is not reading, frames wait.

void startUsbLink();
