//! The set of modules this build knows how to load.
//!
//! # Why a compiled-in table
//!
//! A loaded module is trusted native code in this process: it shares the address
//! space, the privileges and the crash domain, and tinybus never unloads it.
//! Which modules may be loaded, and which bytes count as legitimate, are
//! therefore build-time decisions rather than runtime discovery. There is no
//! "module marketplace" here on purpose — a registry a server could add entries
//! to would be a remote-code-execution surface with a download step.
//!
//! # The digests are a second gate, not the only one
//!
//! tinybus fetches the release's own `checksum.toml`, compares it with the digest
//! the host supplies, hashes the downloaded archive, and only then extracts and
//! loads. The digests below are the host's half of that agreement. Pinning them
//! in the source is what makes the check auditable offline: a reviewer can read
//! this file against the release page, and a release re-cut under the same tag
//! stops matching rather than silently replacing what runs in-process.
//!
//! # Adding an entry
//!
//! Take the values verbatim from the release's `checksum.toml`. Do not compute
//! them from a local build — the point is to pin what the release publishes, and
//! a locally recomputed digest would agree with itself no matter what was served.

use super::types::{LoadPolicy, ModuleRecord, PlatformAsset};

/// The `tinydocs` module: `.docx` / `.pptx` synthesis and `.pdf` extraction.
///
/// Lazy, because a user who never asks for a document should not pay a download,
/// a `dlopen`, and the resident cost of a library that is never unloaded.
const TINYDOCS: ModuleRecord = ModuleRecord {
    id: "tinydocs",
    description: "Document synthesis (.docx, .pptx) and PDF text extraction",
    bus_name: "ai.tinyhumans.tinydocs.Documents",
    object_path: "/ai/tinyhumans/tinydocs/Documents",
    version: "0.1.11",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.1.11",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.1.11-ubuntu-24.04-x86_64.tar.gz",
            sha256: "675133a527db29ba2b954372df71a705f07ac779a027e627bf442165b6045c26",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.1.11-ubuntu-24.04-arm64.tar.gz",
            sha256: "31f83410d7db07bc011ed168cfc61f124f83881c2d4bf1707ee062fbb51ba6d6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.1.11-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c87a32f32ab61f7969680a8d7d1d46f216c9cdd086b6d22c8574a478a5bf0d27",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.1.11-ubuntu-22.04-arm64.tar.gz",
            sha256: "8a799e56c08a86d80bdd5c7796a9c4fde25b70c8520751091978fc640c9b7af7",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.1.11-macos-26-arm64.tar.gz",
            sha256: "d29aa3dec8495b60ebeeef8a8f35a5a4466b7db7b5b09563e465a7746e8f9dd1",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.1.11-macos-26-x86_64.tar.gz",
            sha256: "22eadd926d492528afe86478de6cd880471279854c83626b1c2baa718a66aa71",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.1.11-macos-15-arm64.tar.gz",
            sha256: "3f0d654bc2a792fedf1e3727679738cd84f40af8554b8cdf89e1f20335365253",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.1.11-macos-15-x86_64.tar.gz",
            sha256: "f765e740e27f5a73b86a60ab8a7c760681afb1ee0061ef72331ad9909253fcab",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.1.11-windows-2025-x86_64.zip",
            sha256: "60ce16ea52b25a205a874a0a98fd760f1e79b422d90a4c6236a16385ca44b561",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.1.11-windows-2022-x86_64.zip",
            sha256: "875d9694c9efb49d8b70adff5071198227103105fda32bb8a4440b08dade6a55",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.1.11-windows-11-arm64.zip",
            sha256: "cf44d9d9466f7294421d28f156df0bfbdbd25a15810896dcebdb9d9bdb939978",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// Every module this build can load.
pub const ALL: &[ModuleRecord] = &[TINYDOCS];

/// The record for `id`, if this build knows it.
#[must_use]
pub fn find(id: &str) -> Option<&'static ModuleRecord> {
    ALL.iter().find(|record| record.id == id)
}

#[cfg(test)]
mod tests {
    use super::{find, ALL};
    use crate::openhuman::modules::platform::candidates_for;

    #[test]
    fn ids_and_bus_names_are_unique() {
        // Two records claiming one bus name is a conflict tinybus would only
        // surface at load time, on whichever one happened to be second.
        for (i, record) in ALL.iter().enumerate() {
            for other in &ALL[i + 1..] {
                assert_ne!(record.id, other.id, "duplicate module id");
                assert_ne!(record.bus_name, other.bus_name, "duplicate bus name");
            }
        }
    }

    #[test]
    fn every_object_path_matches_its_bus_name() {
        // tinybus derives a module's object path from its bus name by replacing
        // dots with slashes, and admission compares the two. A mismatch here is
        // a module that downloads and then refuses to load.
        for record in ALL {
            assert_eq!(
                record.object_path,
                format!("/{}", record.bus_name.replace('.', "/")),
                "{} object path does not match its bus name",
                record.id
            );
        }
    }

    #[test]
    fn every_digest_is_a_lowercase_sha256() {
        // An uppercase or truncated digest is refused by tinybus at download
        // time, which is a slow way to find a typo in this file.
        for record in ALL {
            for asset in record.assets {
                assert_eq!(
                    asset.sha256.len(),
                    64,
                    "{} / {} digest is not 64 characters",
                    record.id,
                    asset.host_key
                );
                assert!(
                    asset
                        .sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "{} / {} digest is not lowercase hex",
                    record.id,
                    asset.host_key
                );
            }
        }
    }

    #[test]
    fn every_asset_name_carries_its_host_key_and_a_known_extension() {
        // tinybus selects the asset by exact name and requires a `.tar.gz` or
        // `.zip` archive, so a name that does not match its key is a module that
        // loads the wrong platform's library.
        for record in ALL {
            for asset in record.assets {
                assert!(
                    asset.archive.contains(asset.host_key),
                    "{} asset {} does not name its host key {}",
                    record.id,
                    asset.archive,
                    asset.host_key
                );
                let windows = asset.host_key.starts_with("windows");
                assert_eq!(
                    windows,
                    asset.archive.ends_with(".zip"),
                    "{} asset {} has the wrong archive format for its host",
                    record.id,
                    asset.archive
                );
                if !windows {
                    assert!(asset.archive.ends_with(".tar.gz"));
                }
            }
        }
    }

    #[test]
    fn every_asset_name_carries_the_pinned_version() {
        // The digests and the version have to describe one release; an asset
        // left behind at an older version would download bytes the digest
        // beside it never matched.
        for record in ALL {
            for asset in record.assets {
                assert!(
                    asset.archive.contains(record.version),
                    "{} asset {} is not from version {}",
                    record.id,
                    asset.archive,
                    record.version
                );
            }
        }
    }

    #[test]
    fn the_release_url_is_a_tag_on_github() {
        // tinybus refuses a URL that is not a tag, because a branch URL names
        // bytes that can change under a digest that was checked once.
        for record in ALL {
            assert!(
                record
                    .release_url
                    .starts_with("https://github.com/tinyhumansai/"),
                "{} release url is not an upstream GitHub URL",
                record.id
            );
            assert!(
                record.release_url.contains("/releases/tag/"),
                "{} release url is not a tag",
                record.id
            );
            assert!(
                record.release_url.ends_with(record.version),
                "{} release url does not name version {}",
                record.id,
                record.version
            );
        }
    }

    #[test]
    fn every_host_the_platform_table_can_produce_has_an_asset() {
        // The two tables are written independently and would drift silently:
        // `platform` offering a key no release publishes turns a supported host
        // into an "unsupported host" at first use.
        let hosts = [
            ("linux", "x86_64", Some((2, 39))),
            ("linux", "aarch64", Some((2, 39))),
            ("linux", "x86_64", Some((2, 35))),
            ("linux", "aarch64", Some((2, 35))),
            ("macos", "x86_64", None),
            ("macos", "aarch64", None),
            ("windows", "x86_64", None),
            ("windows", "aarch64", None),
        ];
        for record in ALL {
            for (os, arch, glibc) in hosts {
                for key in candidates_for(os, arch, glibc) {
                    assert!(
                        record.asset_for(&key).is_some(),
                        "{} publishes no asset for {key}, which {os}/{arch} would ask for",
                        record.id
                    );
                }
            }
        }
    }

    #[test]
    fn find_resolves_known_ids_only() {
        assert!(find("tinydocs").is_some());
        assert!(find("not-a-module").is_none());
    }
}
