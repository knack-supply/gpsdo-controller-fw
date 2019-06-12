pub type LFSR32 = lfsr::galois::Galois32;

lfsr::lfsr_lookup!(
    reverse_sig_int,
    lfsr::galois::Galois32,
    9_999_900,
    10_000_100,
    50
);

pub fn reverse_sig(lfsr: LFSR32) -> Option<u32> {
    reverse_sig_int(&lfsr)
}

#[cfg(feature = "hx8k")]
lfsr::lfsr_lookup!(
    reverse_clk_int,
    lfsr::galois::Galois32,
    200_999_000,
    201_001_000,
    50
);
#[cfg(feature = "up5k")]
lfsr::lfsr_lookup!(
    reverse_clk_int,
    lfsr::galois::Galois32,
    100_499_000,
    100_501_000,
    50
);

pub fn reverse_clk(lfsr: LFSR32) -> Option<u32> {
    reverse_clk_int(&lfsr)
}
