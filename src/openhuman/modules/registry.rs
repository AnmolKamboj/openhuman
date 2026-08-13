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
    version: "0.1.13",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.1.13",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.1.13-ubuntu-24.04-x86_64.tar.gz",
            sha256: "43ad43b0fea00de3f82f960c5eae297b528334780905286f683857cbd7e7fa07",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.1.13-ubuntu-24.04-arm64.tar.gz",
            sha256: "66a4d9a4cb1caea86fe6203cde54db06165d483c59e8f86b61439f257be7dff8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.1.13-ubuntu-22.04-x86_64.tar.gz",
            sha256: "3e3a7c2e774d75654a7e9074e41ad972a670f2a0dcf8ee2648dfdbb404edc7cb",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.1.13-ubuntu-22.04-arm64.tar.gz",
            sha256: "12f0c83a6239423be9001ec57cf9d53a50c639e3d67449646f48a9eef207f36b",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.1.13-macos-26-arm64.tar.gz",
            sha256: "6a8edb36258a241c62497dd962c3690f0f287944663a7edc00602e652ac72298",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.1.13-macos-26-x86_64.tar.gz",
            sha256: "dfcd0f79f6ea9ffd7c9f510f4007285a0cc7d434ddf286a9dc870468003d3784",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.1.13-macos-15-arm64.tar.gz",
            sha256: "8b1be8ac2db781fd0ff8af8815e6dd408d79fd8c489032358447434a21bdf52a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.1.13-macos-15-x86_64.tar.gz",
            sha256: "c84dcf6b3fc4eac5985b56297e35eb730dc86c7717fdfe72886f9c189efc22ba",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.1.13-windows-2025-x86_64.zip",
            sha256: "30a0ef74959029ed385ee4a3e47f8f42bd4eeeb12c2d95030107fa7ac16d5dbe",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.1.13-windows-2022-x86_64.zip",
            sha256: "f8a7097166074aff712e6847207c112f3afcc95a6a875177bcc167b46cd6d332",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.1.13-windows-11-arm64.zip",
            sha256: "366f92165c1a3ef4361568edacb0ca4053a0209efbf804730ab35ee37b743ee7",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinywallet` module: transaction building and assembly for four chains.
///
/// Lazy for the same reason as [`TINYDOCS`], and more so: most sessions never
/// touch a wallet, and this artifact carries `bitcoin` and a native `secp256k1`
/// build that would otherwise be resident for all of them.
///
/// **The signing key is never sent to this module.** It returns the bytes that
/// need signing and reassembles once this process has signed them — see
/// [`super::wallet`]. Nothing in its interface accepts key material.
const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: "ai.tinyhumans.tinywallet.Wallet",
    object_path: "/ai/tinyhumans/tinywallet/Wallet",
    version: "0.2.1",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.2.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.2.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "42e3440d367c251d687505115b7c0e2bdcd8ec4f064438a03762bbb5c1b651c0",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.2.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "9b9f102e1bde35bc59ca9898c0531de87dc34408a6d495a56d821e173284f65a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.2.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "77c3f8a188ac69d4faa4b7305c83065682b36683754e3072e8f6a1d64c8fe795",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.2.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "1a63b576eb5f07dd54cfd27f63f3f6c86718fb93e11726ec6482d7cc95db6863",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.2.1-macos-26-arm64.tar.gz",
            sha256: "3c89b41511156ced51267da77a83519d7fae0ab10b2efa311d1a5ffde748a360",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.2.1-macos-26-x86_64.tar.gz",
            sha256: "eb3ea578e0b05f03a8150af07de40b4c61b584e0d1c1944b2172bde8a356701c",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.2.1-macos-15-arm64.tar.gz",
            sha256: "768a6eb74ceff9ddcc6c7d0c79dc2942e29d4264a7df11d82152c921b426aa5a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.2.1-macos-15-x86_64.tar.gz",
            sha256: "c9ee8e0367beb56ef9aafbe4d830c4e4a58b4b6d0150f7f42c4fb4536441099c",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.2.1-windows-2025-x86_64.zip",
            sha256: "ef64bd36086fcba105f30703bc3ef7102a24a48e510e622c502b8b9bfa2ea68f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.2.1-windows-2022-x86_64.zip",
            sha256: "1d9f18071ee185a8b13ffc6c93e0f83c232eac879dca46bb5d36d9e832a133e4",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.2.1-windows-11-arm64.zip",
            sha256: "20ed7d288fcd9d8a58eb774a30099a292bff06bc1d5b983a06b88d555e0b41dc",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The complete TinyMemory engine, loaded eagerly so its capabilities are
/// available when the kernel assembles its RPC and tool surfaces.
const TINYMEMORY: ModuleRecord = ModuleRecord {
    id: "tinymemory",
    description: "Local memory engine: store, ranked recall, and portable export",
    bus_name: "ai.tinyhumans.tinymemory.Memory",
    object_path: "/ai/tinyhumans/tinymemory/Memory",
    version: "1.0.1",
    release_url: "https://github.com/tinyhumansai/tinymemory/releases/tag/v1.0.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymemory-module-1.0.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "723bb5b006c6f45258e3176a944beb70524f884eb1abe21e4c7a8747058e32a3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymemory-module-1.0.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "34af9da10d143c5f5ddc13f969da2119bfe89b309acab5bdc2a9c24c7f341706",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymemory-module-1.0.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "35d13463041f455bebd833a71ba370d891a4e59d1b8e576e81635e2777a0c3dd",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymemory-module-1.0.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "39f4cad7d781e3feb30dead75bccc3b0c432f095a88f4a73a3fa2d40db08c861",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymemory-module-1.0.1-macos-26-arm64.tar.gz",
            sha256: "de27f5eb1510e10c558f4856448eb0c07b6f79d0b345be777a299aa334259245",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymemory-module-1.0.1-macos-26-x86_64.tar.gz",
            sha256: "a9bceaf3a9f72708bd0ee8a3ed10c0bd25c01d13296265cb12382cf9ad443f10",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymemory-module-1.0.1-macos-15-arm64.tar.gz",
            sha256: "dcb0d7ce49b769f51f231fb864975898c4ba94a53f1d38d53925cabb843e1efd",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymemory-module-1.0.1-macos-15-x86_64.tar.gz",
            sha256: "cdd4fc69898cc526461aaabbd5b75ddca2d2b180873c079682ca1c4e77442b89",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymemory-module-1.0.1-windows-2025-x86_64.zip",
            sha256: "972d15fa8cc3fe401d25a26b8bcc349470831cec5a6e7ab60a546a7f3f130887",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymemory-module-1.0.1-windows-2022-x86_64.zip",
            sha256: "c9c08ba1ee9c60484fc775b54a16954d544b71bbb96e1b604ccdd40327aceec9",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymemory-module-1.0.1-windows-11-arm64.zip",
            sha256: "47e956ce93ed4f8cea5ab74cc54eb277ac84ebc0ff9939e2b1003f1a0d009a73",
        },
    ],
    // Eager, unlike the two codecs above. A codec that is never asked for should
    // not be paid for, but a memory driver's absence changes what the kernel
    // offers rather than merely delaying it: capabilities are read at bind time
    // and the RPC surface and agent-tool list are filtered from them. Resolving
    // that during a user's first recall would mean the first recall is the one
    // that behaves differently.
    load: LoadPolicy::Eager,
};

/// Every module this build can load.
pub const ALL: &[ModuleRecord] = &[TINYDOCS, TINYWALLET, TINYMEMORY];

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
