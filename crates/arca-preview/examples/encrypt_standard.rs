// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Writes a password-protected Office package for the test fixtures: "standard" encryption
//! (Office 2007, AES-128 in ECB mode, MS-OFFCRYPTO 2.3.4), built from the specification.
//!
//! ```sh
//! cargo run -p arca-preview --example encrypt_standard -- \
//!     tests/fixtures/sheet.xlsx tests/fixtures/enc-default.xlsx VelvetSweatshop
//! cargo run -p arca-preview --example encrypt_standard -- \
//!     tests/fixtures/report.docx tests/fixtures/enc-smuggle.docx s3cret --smuggle
//! ```
//!
//! `--smuggle` puts streams next to the encrypted package that are not part of the encryption:
//! what a file looks like when someone hides a second payload behind the password prompt. The
//! payload is inert text. Nothing here is, or contains, malware.

#![allow(clippy::print_stdout, clippy::print_stderr, missing_docs)]

use arca_container::cfb::writer::CfbBuilder;
use arca_container::crypto::aes::Aes;
use arca_container::crypto::sha1::sha1;

const KEY_BYTES: usize = 16;
const SPIN: u32 = 50_000;

fn utf16le(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// The key of MS-OFFCRYPTO 2.3.4.7 for AES-128 with SHA-1.
fn derive_key(password: &str, salt: &[u8; 16]) -> Vec<u8> {
    let mut first = salt.to_vec();
    first.extend(utf16le(password));
    let mut h = sha1(&first);
    for i in 0..SPIN {
        let mut block = i.to_le_bytes().to_vec();
        block.extend_from_slice(&h);
        h = sha1(&block);
    }
    let mut last = h.to_vec();
    last.extend_from_slice(&0u32.to_le_bytes());
    let h_final = sha1(&last);
    let pad = |byte: u8| -> [u8; 20] {
        let mut b = [byte; 64];
        for (x, y) in b.iter_mut().zip(h_final.iter()) {
            *x ^= y;
        }
        sha1(&b)
    };
    let mut derived = pad(0x36).to_vec();
    derived.extend_from_slice(&pad(0x5C));
    derived.truncate(KEY_BYTES);
    derived
}

fn ecb_encrypt(aes: &Aes, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    for chunk in data.chunks(16) {
        let mut block = [0u8; 16];
        for (dst, src) in block.iter_mut().zip(chunk) {
            *dst = *src;
        }
        aes.encrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    out
}

fn build(plain: &[u8], password: &str, smuggle: bool) -> Option<Vec<u8>> {
    let salt: [u8; 16] = std::array::from_fn(|i| 0x10 + u8::try_from(i).unwrap_or(0));
    let verifier: [u8; 16] = std::array::from_fn(|i| 0x40 + u8::try_from(i).unwrap_or(0));
    let aes = Aes::new(&derive_key(password, &salt))?;

    let csp = utf16le("Microsoft Enhanced RSA and AES Cryptographic Provider\0");
    let mut header = Vec::new();
    for v in [0x24u32, 0, 0x660E, 0x8004, 128, 0x18, 0, 0] {
        header.extend_from_slice(&v.to_le_bytes());
    }
    header.extend_from_slice(&csp);

    let mut info = Vec::new();
    info.extend_from_slice(&4u16.to_le_bytes());
    info.extend_from_slice(&2u16.to_le_bytes());
    info.extend_from_slice(&0x24u32.to_le_bytes());
    info.extend_from_slice(&u32::try_from(header.len()).ok()?.to_le_bytes());
    info.extend_from_slice(&header);
    info.extend_from_slice(&16u32.to_le_bytes());
    info.extend_from_slice(&salt);
    info.extend_from_slice(&ecb_encrypt(&aes, &verifier));
    info.extend_from_slice(&20u32.to_le_bytes());
    let mut hash = sha1(&verifier).to_vec();
    hash.resize(32, 0);
    info.extend_from_slice(&ecb_encrypt(&aes, &hash));

    let mut package = u64::try_from(plain.len()).ok()?.to_le_bytes().to_vec();
    package.extend_from_slice(&ecb_encrypt(&aes, plain));

    let mut cfb = CfbBuilder::new();
    cfb.add_stream("EncryptionInfo", info).ok()?;
    cfb.add_stream("EncryptedPackage", package).ok()?;
    if smuggle {
        // A second document for a reader that ignores the encryption: a Word stream set with a
        // macro project. Inert filler, shaped only by its stream names.
        cfb.add_stream("WordDocument", vec![0u8; 4096]).ok()?;
        cfb.add_storage("Macros", [0u8; 16]).ok()?;
        cfb.add_storage("Macros/VBA", [0u8; 16]).ok()?;
        cfb.add_stream("Macros/VBA/dir", b"inert test filler, not a project".repeat(8)).ok()?;
        cfb.add_stream("Macros/VBA/_VBA_PROJECT", vec![0xCC, 0x61, 0xFF, 0xFF, 0, 0, 0]).ok()?;
    }
    cfb.finish().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input, output, password, smuggle) = match args.as_slice() {
        [i, o, p] => (i, o, p, false),
        [i, o, p, flag] if flag == "--smuggle" => (i, o, p, true),
        _ => {
            eprintln!("usage: encrypt_standard <package> <out> <password> [--smuggle]");
            return;
        }
    };
    let plain = match std::fs::read(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{input}: {e}");
            return;
        }
    };
    let Some(data) = build(&plain, password, smuggle) else {
        eprintln!("could not build the package");
        return;
    };
    match arca_preview::decrypt(&data, password.as_bytes()) {
        Ok(d) if d.bytes == plain => {}
        other => {
            eprintln!("the package does not decrypt back: {:?}", other.map(|d| d.bytes.len()));
            return;
        }
    }
    if let Err(e) = std::fs::write(output, &data) {
        eprintln!("{output}: {e}");
        return;
    }
    println!("{output}: {} bytes", data.len());
}
