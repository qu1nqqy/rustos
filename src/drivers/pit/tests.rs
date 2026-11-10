use super::*;

#[test_case]
fn test_divisor_for_common_frequencies() {
    // 1193182 / 100 = 11931.82, округляется до 11932
    assert_eq!(divisor(100), 11932);
    assert_eq!(divisor(1000), 1193);
    assert_eq!(divisor(50), 23864);
}

#[test_case]
fn test_divisor_for_max_frequency() {
    assert_eq!(divisor(BASE_FREQUENCY), 1);
    assert_eq!(divisor(u32::MAX), 1);
}

#[test_case]
fn test_divisor_for_low_frequencies_is_maximum() {
    // ниже ≈18.2 Гц делитель не помещается в 16 бит: самый большой (65536) пишется как 0
    assert_eq!(divisor(18), 0);
    assert_eq!(divisor(1), 0);
    assert_eq!(divisor(0), 0);
}

#[test_case]
fn test_divisor_near_minimum_frequency() {
    // 1193182 / 19 = 62799.05
    assert_eq!(divisor(19), 62799);
}

#[test_case]
fn test_frequency_from_divisor() {
    assert_eq!(frequency(11932), 99);
    assert_eq!(frequency(1193), 1000);
    assert_eq!(frequency(1), BASE_FREQUENCY);
    // 0 - это 65536, частота по умолчанию после включения
    assert_eq!(frequency(0), 18);
}

#[test_case]
fn test_command_byte() {
    // канал 0, младший и старший байт, режим 3, двоичный счёт
    assert_eq!(COMMAND_CHANNEL_0_SQUARE_WAVE >> 6, 0);
    assert_eq!(COMMAND_CHANNEL_0_SQUARE_WAVE >> 4 & 0b11, 0b11);
    assert_eq!(COMMAND_CHANNEL_0_SQUARE_WAVE >> 1 & 0b111, 3);
    assert_eq!(COMMAND_CHANNEL_0_SQUARE_WAVE & 1, 0);
}
