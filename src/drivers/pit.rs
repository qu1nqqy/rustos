use x86_64::instructions::port::Port;

/// Частота, с которой трактуется PIT: 1.193182 МГц, так сложилось в IBM PC
pub const BASE_FREQUENCY: u32 = 1_193_182;

// Порт 0x43 - команды, 0x40 - данные канала 0. Канал 0 подключен к IRQ0
const COMMAND_PORT: u16 = 0x43;
const CHANNEL_0_PORT: u16 = 0x40;
// 00 - канал 0, 11 - делитель пишется двумя байтами
// 011 - режим 3 (прерывание раз в период), 0 - делитель двоичный
const COMMAND_CHANNEL_0_SQUARE_WAVE: u8 = 0b0011_0110;

pub fn divisor(hz: u32) -> u16 {
    let hz = hz.clamp(1, BASE_FREQUENCY);
    let divisor = (BASE_FREQUENCY + hz / 2) / hz;
    match divisor {
        65536.. => 0,
        divisor => divisor as u16,
    }
}

pub fn frequency(divisor: u16) -> u32 {
    let divisor = if divisor == 0 { 65536 } else { divisor as u32 };
    BASE_FREQUENCY / divisor
}

pub fn init(hz: u32) {
    let divisor = divisor(hz);
    let mut command = Port::<u8>::new(COMMAND_PORT);
    let mut data = Port::<u8>::new(CHANNEL_0_PORT);
    unsafe {
        command.write(COMMAND_CHANNEL_0_SQUARE_WAVE);
        data.write(divisor as u8);
        data.write((divisor >> 8) as u8);
    }
}

#[cfg(test)]
mod tests;
