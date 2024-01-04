flash:
    cargo build
    espflash flash "target/xtensa-esp32-espidf/debug/artnet-esp32"

monitor:
    espflash monitor