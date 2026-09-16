// MurmurHash2 64A (spaCy strings) and Thinc's uint64 MurmurHash3 specialization.
pub fn hash(s: &str) -> u64 {
    let m = 0xc6a4a7935bd1e995u64;
    let mut h = 1u64 ^ (s.len() as u64).wrapping_mul(m);
    let mut chunks = s.as_bytes().chunks_exact(8);
    for c in &mut chunks {
        let mut k = u64::from_le_bytes(c.try_into().unwrap());
        k = k.wrapping_mul(m);
        k ^= k >> 47;
        k = k.wrapping_mul(m);
        h ^= k;
        h = h.wrapping_mul(m);
    }
    let tail = chunks.remainder();
    for (i, b) in tail.iter().enumerate() {
        h ^= (*b as u64) << (8 * i)
    }
    if !tail.is_empty() {
        h = h.wrapping_mul(m)
    }
    h ^= h >> 47;
    h = h.wrapping_mul(m);
    h ^= h >> 47;
    h
}
pub fn keys(value: u64, seed: u64) -> [u32; 4] {
    fn mix(mut h: u64) -> u64 {
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^ (h >> 33)
    }
    let mut a = value
        .wrapping_mul(0x87c37b91114253d5)
        .rotate_left(31)
        .wrapping_mul(0x4cf5ad432745937f)
        ^ seed
        ^ 8;
    let mut b = seed ^ 8;
    a = a.wrapping_add(b);
    b = b.wrapping_add(a);
    a = mix(a);
    b = mix(b);
    a = a.wrapping_add(b);
    b = b.wrapping_add(a);
    [a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32]
}
