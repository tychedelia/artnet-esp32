flash:
    CRATE_CC_NO_DEFAULTS=1 cargo build --release
    cargo espflash flash --release

monitor:
    cargo espflash monitor