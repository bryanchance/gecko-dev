#![cfg_attr(feature = "deny-warnings", deny(warnings))]
#![warn(clippy::pedantic)]
#![cfg(not(feature = "fuzzing"))]

use neqo_crypto::constants::{Cipher, TLS_AES_128_GCM_SHA256, TLS_VERSION_1_3};
use neqo_crypto::hkdf;
use neqo_crypto::Aead;
use test_fixture::fixture_init;

const AAD: &[u8] = &[
    0xc1, 0xff, 0x00, 0x00, 0x12, 0x05, 0xf0, 0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62, 0xb5, 0x00, 0x40,
    0x74, 0x00, 0x01,
];
const PLAINTEXT: &[u8] = &[
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x18, 0x41, 0x0a, 0x02, 0x00, 0x00, 0x56, 0x03, 0x03, 0xee, 0xfc,
    0xe7, 0xf7, 0xb3, 0x7b, 0xa1, 0xd1, 0x63, 0x2e, 0x96, 0x67, 0x78, 0x25, 0xdd, 0xf7, 0x39, 0x88,
    0xcf, 0xc7, 0x98, 0x25, 0xdf, 0x56, 0x6d, 0xc5, 0x43, 0x0b, 0x9a, 0x04, 0x5a, 0x12, 0x00, 0x13,
    0x01, 0x00, 0x00, 0x2e, 0x00, 0x33, 0x00, 0x24, 0x00, 0x1d, 0x00, 0x20, 0x9d, 0x3c, 0x94, 0x0d,
    0x89, 0x69, 0x0b, 0x84, 0xd0, 0x8a, 0x60, 0x99, 0x3c, 0x14, 0x4e, 0xca, 0x68, 0x4d, 0x10, 0x81,
    0x28, 0x7c, 0x83, 0x4d, 0x53, 0x11, 0xbc, 0xf3, 0x2b, 0xb9, 0xda, 0x1a, 0x00, 0x2b, 0x00, 0x02,
    0x03, 0x04,
];

fn make_aead(cipher: Cipher) -> Aead {
    fixture_init();

    let secret = hkdf::import_key(
        TLS_VERSION_1_3,
        &[
            0x47, 0xb2, 0xea, 0xea, 0x6c, 0x26, 0x6e, 0x32, 0xc0, 0x69, 0x7a, 0x9e, 0x2a, 0x89,
            0x8b, 0xdf, 0x5c, 0x4f, 0xb3, 0xe5, 0xac, 0x34, 0xf0, 0xe5, 0x49, 0xbf, 0x2c, 0x58,
            0x58, 0x1a, 0x38, 0x11,
        ],
    )
    .expect("make a secret");
    Aead::new(
        false,
        TLS_VERSION_1_3,
        cipher,
        &secret,
        "quic ", // QUICv1 label prefix; note the trailing space here.
    )
    .expect("can make an AEAD")
}

#[test]
fn aead_encrypt_decrypt() {
    const TOGGLE: u8 = 77;
    let aead = make_aead(TLS_AES_128_GCM_SHA256);
    let ciphertext_buf = &mut [0; 1024]; // Can't use PLAINTEXT.len() here.
    let ciphertext = aead
        .encrypt(1, AAD, PLAINTEXT, ciphertext_buf)
        .expect("encrypt should work");
    let expected_ciphertext: &[u8] = &[
        0x5f, 0x01, 0xc4, 0xc2, 0xa2, 0x30, 0x3d, 0x29, 0x7e, 0x3c, 0x51, 0x9b, 0xf6, 0xb2, 0x23,
        0x86, 0xe3, 0xd0, 0xbd, 0x6d, 0xfc, 0x66, 0x12, 0x16, 0x77, 0x29, 0x80, 0x31, 0x04, 0x1b,
        0xb9, 0xa7, 0x9c, 0x9f, 0x0f, 0x9d, 0x4c, 0x58, 0x77, 0x27, 0x0a, 0x66, 0x0f, 0x5d, 0xa3,
        0x62, 0x07, 0xd9, 0x8b, 0x73, 0x83, 0x9b, 0x2f, 0xdf, 0x2e, 0xf8, 0xe7, 0xdf, 0x5a, 0x51,
        0xb1, 0x7b, 0x8c, 0x68, 0xd8, 0x64, 0xfd, 0x3e, 0x70, 0x8c, 0x6c, 0x1b, 0x71, 0xa9, 0x8a,
        0x33, 0x18, 0x15, 0x59, 0x9e, 0xf5, 0x01, 0x4e, 0xa3, 0x8c, 0x44, 0xbd, 0xfd, 0x38, 0x7c,
        0x03, 0xb5, 0x27, 0x5c, 0x35, 0xe0, 0x09, 0xb6, 0x23, 0x8f, 0x83, 0x14, 0x20, 0x04, 0x7c,
        0x72, 0x71, 0x28, 0x1c, 0xcb, 0x54, 0xdf, 0x78, 0x84,
    ];
    assert_eq!(ciphertext, expected_ciphertext);

    let plaintext_buf = &mut [0; 1024]; // Can't use PLAINTEXT.len() here.
    let plaintext = aead
        .decrypt(1, AAD, ciphertext, plaintext_buf)
        .expect("decrypt should also work");
    assert_eq!(plaintext, PLAINTEXT);

    // Decryption failures...
    // Different counter.
    let res = aead.decrypt(2, AAD, ciphertext, plaintext_buf);
    assert!(res.is_err());

    // Front-truncate ciphertext.
    let res = aead.decrypt(1, AAD, &ciphertext[1..], plaintext_buf);
    assert!(res.is_err());

    // End-truncate ciphertext.
    let ciphertext_last = ciphertext.len() - 1;
    let res = aead.decrypt(1, AAD, &ciphertext[..ciphertext_last], plaintext_buf);
    assert!(res.is_err());

    // Mess with the buffer.
    let mut scratch = Vec::new();
    scratch.extend_from_slice(ciphertext);

    // Toggle first octet.
    scratch[0] ^= TOGGLE;
    let res = aead.decrypt(1, AAD, &scratch[..], plaintext_buf);
    assert!(res.is_err());

    // Toggle the auth tag.
    scratch[0] ^= TOGGLE;
    scratch[ciphertext_last] ^= TOGGLE;
    let res = aead.decrypt(1, AAD, &scratch[..], plaintext_buf);
    assert!(res.is_err());

    // Mess with the AAD.
    scratch.clear();
    scratch.extend_from_slice(AAD);

    // Front-truncate.
    let res = aead.decrypt(1, &scratch[1..], ciphertext, plaintext_buf);
    assert!(res.is_err());

    // End-truncate.
    let aad_last = AAD.len() - 1;
    let res = aead.decrypt(1, &scratch[..aad_last], ciphertext, plaintext_buf);
    assert!(res.is_err());

    scratch[0] ^= TOGGLE;
    let res = aead.decrypt(1, &scratch[..], ciphertext, plaintext_buf);
    assert!(res.is_err());
}
