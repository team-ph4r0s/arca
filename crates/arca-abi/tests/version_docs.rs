// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! The crate description and the README name the current ABI version, so a version bump cannot
//! leave them behind.

use arca_abi::ABI_VERSION;

#[test]
fn the_crate_description_names_the_current_abi_version() {
    let description = env!("CARGO_PKG_DESCRIPTION");
    let want = format!("ABI v{ABI_VERSION}:");
    assert!(
        description.contains(&want),
        "crates/arca-abi/Cargo.toml description must say {want:?}: {description:?}"
    );
}

#[test]
fn the_readme_names_the_current_abi_version() {
    let readme = include_str!("../../../README.md");
    for want in [
        format!("(ABI {ABI_VERSION})"),
        format!("(ABI v{ABI_VERSION})"),
        format!("ABI-v{ABI_VERSION}-"),
    ] {
        assert!(readme.contains(&want), "README.md must say {want:?} (the current ABI_VERSION)");
    }
    for old in 1..ABI_VERSION {
        for stale in [format!("(ABI {old})"), format!("(ABI v{old})"), format!("ABI-v{old}-")] {
            assert!(
                !readme.contains(&stale),
                "README.md still says {stale:?}; the ABI is version {ABI_VERSION}"
            );
        }
    }
}
