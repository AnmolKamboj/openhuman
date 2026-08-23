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
    version: "0.1.14",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.1.14",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.1.14-ubuntu-24.04-x86_64.tar.gz",
            sha256: "2dfee3d8d9322474114bf3bc1775f57ed7f8258d53c11a78fe5302538fdd0d1e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.1.14-ubuntu-24.04-arm64.tar.gz",
            sha256: "0efb5c25babd13fea2c1ef0faef43bc6a06a9b1bd155b145fbdb03dbbe2875fa",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.1.14-ubuntu-22.04-x86_64.tar.gz",
            sha256: "fac4385075e0a1eb1f86355b9b96cae25a3a84bad30417ba3fd417db61ec6385",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.1.14-ubuntu-22.04-arm64.tar.gz",
            sha256: "8f6e77a492668d446a47b65713324300da3e7319a77d6865487a938462528575",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.1.14-macos-26-arm64.tar.gz",
            sha256: "9a086ed43ddfebd80aad4df832f9a996c1fadf46bc60c4f251db4e46b1acb319",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.1.14-macos-26-x86_64.tar.gz",
            sha256: "b43ffddbba88c1e54939419f1eb0f76b65bf6a9411bf12fe6f5929b448dfa51a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.1.14-macos-15-arm64.tar.gz",
            sha256: "9ffad3fd0464e35e66d3958a6f8b7bf2309f4af2ae8ca167b9d653231c47597d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.1.14-macos-15-x86_64.tar.gz",
            sha256: "f26e3bb312af83ef6dbf197b7193fc0cfab0ea21438b01de8fb64d290b9d5b0c",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.1.14-windows-2025-x86_64.zip",
            sha256: "212f9822db5ac1698018326ac636224f55543dc7f4608bb06da3880cba71f79b",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.1.14-windows-2022-x86_64.zip",
            sha256: "7922905cce57a2d345fabe15ca4cb6c8d66c4e06edc496e1f096338173eb86a3",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.1.14-windows-11-arm64.zip",
            sha256: "e9664823b4b9ca083968ecc9bb3cb0b932c2288a4df027d21269c34673d040e4",
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
/// **This host sends it the recovery phrase, over confidential calls, and never
/// derives or signs itself.** All four chains — Bitcoin, EVM, Solana and Tron —
/// derive and sign inside the module. This binary links neither `tinywallet`'s
/// `key` feature nor `k256`; see the note on the `tinywallet` dependency.
///
/// The phrase is only sent to a module tinybus has attested *and* whose digest
/// matches one of the entries below — `super::wallet::attested_proxy` checks
/// this table itself rather than trusting that some check happened.
///
/// One call brings key material back: `ExportKey`, used solely for tiny.place's
/// `LocalSigner::from_seed`, which takes a seed and cannot be handed a message
/// to sign instead. Replacing that seam is what it would take to remove it.
///
/// Three releases got here, and the order mattered. v0.2.3 changed no method at
/// all — it was the same module rebuilt against a bus that could attest it.
/// Attestation used to be recorded only from a `modules.toml` beside the
/// artifact, and a release download extracts into a temporary directory that has
/// none, so this module could never be an attested recipient however carefully
/// the digest below was pinned (tinybus#15 fixed that). Only then was it safe
/// for v0.3.0 to add methods that take a secret, and for v0.4.0 to add
/// `SignMessage` for the Solana and x402 encodings the wire contract does not
/// model. Adding them earlier would have made them unreachable in production and
/// reachable in a developer's tree, which is the worst of both.
const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: "ai.tinyhumans.tinywallet.Wallet",
    object_path: "/ai/tinyhumans/tinywallet/Wallet",
    version: "0.4.0",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.4.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.4.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "737a18c258bb9013ad85006433c72a5dc83b94de8f15a0d37723a3b96cf047fa",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.4.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "72217d4f4dc1a2328de08c83d24998cd51729e8157cd2e9cb3b034ec1da2ea94",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.4.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "e7d2d1a40331b5fea1dc9d8870c206d093c756af91790a15e3fcc9fc1b160158",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.4.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "248fd13ba59ab9c00ccd605b60c533aabd41be0f82cd167758524842122510f1",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.4.0-macos-26-arm64.tar.gz",
            sha256: "e6df7dc830d595a63af6864cbec6e3e22e51f35af558e7b62fa655d6b16d0581",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.4.0-macos-26-x86_64.tar.gz",
            sha256: "fd197ac908057b9b5b4c7aef1b86e74ea7369133eff2a4835c310c73e7816a01",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.4.0-macos-15-arm64.tar.gz",
            sha256: "28a56ed94827b46a972c054b07e614684b7217d8f8c69373e93b957de336901b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.4.0-macos-15-x86_64.tar.gz",
            sha256: "2e97717f08efefb90a8be51f389cbf826fb132fc7111f11837e9ee717c527e58",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.4.0-windows-2025-x86_64.zip",
            sha256: "c9393d6c0f171db34298950ad029c21ea6b41f3f77971cf6668ebbd7f34736b7",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.4.0-windows-2022-x86_64.zip",
            sha256: "8ed5e86977f951a8c54dbde82914f6f936d4402564beb30406f4140d4be02872",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.4.0-windows-11-arm64.zip",
            sha256: "7854dfeb1dd04afe99488616e223a0f3ce6d7c671e22f8fbc3089eb0523cbf51",
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

/// The `tinyjuice` content-aware tool-output compression engine.
///
/// Lazy because the host's compaction policy can disable it, and a session that
/// never produces compressible tool output should not pay the download or
/// resident native-library cost.
const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: "ai.tinyhumans.tinyjuice.Compression",
    object_path: "/ai/tinyhumans/tinyjuice/Compression",
    version: "0.2.4",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.2.4",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.2.4-ubuntu-24.04-x86_64.tar.gz",
            sha256: "1427cd37740a6ff512f8743a5753789537a47133e2b3a09513026a275ec633b5",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.2.4-ubuntu-24.04-arm64.tar.gz",
            sha256: "476ed4c41d5078e612d20af814cc36adf44b97a8c877f243fc11eaec283cb624",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.2.4-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f8677b0d8619ac36791408bbee2125e4f3ed586326da68fd1c2de49291c09b01",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.2.4-ubuntu-22.04-arm64.tar.gz",
            sha256: "b406f1041849284ee71332e2bb74169469345cb64f24f005c6f76cf0fb39b655",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.2.4-macos-26-arm64.tar.gz",
            sha256: "816befb360ed56b3e43e868e4fe5b86f832bee2ca9f97c273649ed7323fb262b",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.2.4-macos-26-x86_64.tar.gz",
            sha256: "9558cf2204cb8535103168fba3581e3ed7c36428a0a39e842a8da48b19ed26f6",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.2.4-macos-15-arm64.tar.gz",
            sha256: "c5fd72170af9bc201885b4563afe78bc9fe05635b583a1ae9f897d5512031f7e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.2.4-macos-15-x86_64.tar.gz",
            sha256: "f75f9d460d76ea8b557c26f915d2163769e8a6fa0aeab96c6e74a8c6d63d01a2",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.2.4-windows-2025-x86_64.zip",
            sha256: "5bc28d173497e0fcf088b5a88ceede1f9aff8f8430866439e8a6dbcbb5609e05",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.2.4-windows-2022-x86_64.zip",
            sha256: "518078ff8e7a4f76c4d0feff452e3fe3fd89b74cac048a5ea2de05d47bd3074c",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.2.4-windows-11-arm64.zip",
            sha256: "efb618098cb6a6bef37ad715d1abcbdea54673e410c8cac930b3e7af11bf032c",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinyvoice` module: the host-agnostic half of the voice pipeline.
///
/// Wake-word gating, fast-path command routing, STT hallucination detection,
/// and the capture-side audio work (downmix, resample, silence gate, WAV
/// framing).
///
/// Lazy, and more clearly so than the others: voice is opt-in twice over — a
/// user has to enable dictation or always-on listening before any of this runs
/// — so a session that never speaks should not pay a download or a `dlopen`.
///
/// **The VAD deliberately does not come through here.** A segmenter is driven
/// once per 20 ms frame from inside a `cpal` callback, and a bus round trip at
/// that cadence would cost more than the sixty-line state machine it replaces.
/// `voice::always_on` keeps its own; see [`super::voice`].
const TINYVOICE: ModuleRecord = ModuleRecord {
    id: "tinyvoice",
    description: "Wake-word gating, command routing, hallucination detection, capture audio",
    bus_name: "ai.tinyhumans.tinyvoice.Voice",
    object_path: "/ai/tinyhumans/tinyvoice/Voice",
    version: "0.1.5",
    release_url: "https://github.com/tinyhumansai/tinyvoice/releases/tag/v0.1.5",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyvoice-module-0.1.5-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8d8db0f7ae600be60f7929f7d77272daa262203d1a67656b3b6a56c774b4ff66",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyvoice-module-0.1.5-ubuntu-24.04-arm64.tar.gz",
            sha256: "6bb931a47a8cf120717d2f6829a37c67c731b485fdfcefeaa46c46e0859d5be1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyvoice-module-0.1.5-ubuntu-22.04-x86_64.tar.gz",
            sha256: "1693c95528850d0547ca70b28d7394fe7db9a20c4da70b22ec0b82fcff23c698",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyvoice-module-0.1.5-ubuntu-22.04-arm64.tar.gz",
            sha256: "63101dc92a7e9c65e4609c983d7370b2d5de87f629d8593f6d5878c24fd1f479",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyvoice-module-0.1.5-macos-26-arm64.tar.gz",
            sha256: "034565947f76a524bdfba33bcc121197e766cda9433e659a23e46b218e7a3e37",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyvoice-module-0.1.5-macos-26-x86_64.tar.gz",
            sha256: "08f1e74f35b9ed830cfb01b6339c3466916b1715b549faecc5de8b053e1a5465",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyvoice-module-0.1.5-macos-15-arm64.tar.gz",
            sha256: "4d6f63a802a372cef4de397f5b6d16bd1c703a09444c48288bf5b9cc25633a19",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyvoice-module-0.1.5-macos-15-x86_64.tar.gz",
            sha256: "fe4582e8ea583f333bb7003bdc54bd24aafd602f20d1d091b32d54b923a83423",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyvoice-module-0.1.5-windows-2025-x86_64.zip",
            sha256: "d89e526e62ebf20361635029284d108ec5a4feb07899715a3de01e4bfacdaf43",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyvoice-module-0.1.5-windows-2022-x86_64.zip",
            sha256: "11a7adf1669c7df3b8d9587eb5ca0a601b403d57bf99209c74b117a69fd57a8d",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyvoice-module-0.1.5-windows-11-arm64.zip",
            sha256: "f39eeecfe54ec2eec9b850dbc4190a69e14de220aa671bac6f7cd889670227e9",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinyruntime` module: the runtime router.
///
/// Resolves a language runtime, installs one when the host has none, reuses one
/// when it does, and runs code on a bounded pool of warm interpreter processes.
/// It is a router: on its own it knows no languages, and it routes to the two
/// provider records below.
///
/// Lazy, because a host that never runs a skill, a flow step, or a `node_exec`
/// should not pay a download and a `dlopen` for the ability to.
///
/// The digests below are v0.2.2's, taken verbatim from that release's
/// `checksum.toml`. Until it existed this record carried no assets at all and
/// the module was reachable only from a developer build named by
/// `modules.local` or found on `OPENHUMAN_MODULE_PATH` — so on any machine that
/// had not built it, the runtime domain was a set of tools that could not run.
const TINYRUNTIME: ModuleRecord = ModuleRecord {
    id: "tinyruntime",
    description: "Language runtime resolution, installation, and pooled execution",
    bus_name: "ai.tinyhumans.runtime.Runtime",
    object_path: "/ai/tinyhumans/runtime/Runtime",
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinyruntime/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyruntime-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "61f642e9c952889d12347beeb6399dd7240b599be21219488abc08ad86b70a82",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyruntime-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "99c8ace3a011fa08e5a526cc9c26e62951cc35f0d23512ea19494eb0d677a871",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyruntime-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "8f2e78662d43e8311291f621bbb61a123ab70d9edfd73177f7f6a92bd1c212c7",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyruntime-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "fbab3aa0c1ed44758446098ce6fca88c43344ff5b7ce03b0aa79000555a9f5ad",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyruntime-0.2.2-macos-26-arm64.tar.gz",
            sha256: "e968577c2df7aeac1cde63e0cb4155d79144ac995ed61cb0584f8ba2562ff748",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyruntime-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "c15d9d492f23796a330f5df53ac39730b15d72c6ca8ce1b09a1ac8fdf760d60a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyruntime-0.2.2-macos-15-arm64.tar.gz",
            sha256: "122f4de043a2f252373a2beaf08ff7e91b3da1947f135a24578b3a09a2574656",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyruntime-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "e1dbfe11cea45df0703ec6bfa579de82740effde99f1977898776505a0ab82da",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyruntime-0.2.2-windows-2025-x86_64.zip",
            sha256: "893f0faaa3f4c1a4b530f63faaec7095f8582e55c1e768f4dfe1fe25a42864c4",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyruntime-0.2.2-windows-2022-x86_64.zip",
            sha256: "ebb59a8680782f0e2cd58450e1bf6423eba2839efd29c7a6380cd62e3f3ef9ef",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyruntime-0.2.2-windows-11-arm64.zip",
            sha256: "7b7accfb5758563ca1ce780b815f5a89d5b566efb7a811492432492794d37423",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinyruntime-nodejs` module: the Node.js half of the router's knowledge.
///
/// Answers which host interpreters count, which archive nodejs.org publishes for
/// this machine, where the binaries land, and what a warm Node worker is. It
/// installs nothing itself.
///
/// It implements the shared `ai.tinyhumans.runtime.Provider` interface but
/// serves at its own object path, because two modules cannot claim one bus name
/// and tinybus derives the path from the name.
///
/// Lazy, and loaded by the same call that loads the router: a language is only
/// worth its `dlopen` when something asks for that language.
///
/// Released alongside the router and pinned the same way — see [`TINYRUNTIME`].
const TINYRUNTIME_NODEJS: ModuleRecord = ModuleRecord {
    id: "tinyruntime-nodejs",
    description: "Node.js runtime provider for tinyruntime",
    bus_name: "ai.tinyhumans.runtime.nodejs.Provider",
    object_path: "/ai/tinyhumans/runtime/nodejs/Provider",
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinyruntime-nodejs/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "60bebfacfaccc5c899044fe542a07b1b2ef74ffeeca5d7f53ef0338b6dab4865",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyruntime-nodejs-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "ff9114e32db29de2a43df83e7d8b330926d5862cdb50ca20adc863d5d99becaf",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "3f25a17d41226fa8cc56cd9f5f5bd447bff4b9f55c1bd68d7bf8ebbf10575aaa",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyruntime-nodejs-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "ec271b78487caaea5c5ae1951568a838be49b5df4d362d8855cb27ba243a8c44",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyruntime-nodejs-0.2.2-macos-26-arm64.tar.gz",
            sha256: "394d160e8de754e09121a52ae6a4b5a7b440c0035fb52cbdaa2dfe7ee523b7b0",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "bbde43f8d839aacb34f735bbde2e8f56207a1a49fb5b07732a3be7b486243ce3",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyruntime-nodejs-0.2.2-macos-15-arm64.tar.gz",
            sha256: "83ea9c8ea1b43dc4e98cb585e98d254080c2070092b3c1458f19012df5ea3cd8",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "6bdb686d1e857d6c28a49ab2ab87785d8c4fecbf7ef62ad218d7b3e159e2339a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-windows-2025-x86_64.zip",
            sha256: "36aab2547fbb7f336e15ecb66768661a4bd35f3da6179fc3efcd47bbb8d0df96",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyruntime-nodejs-0.2.2-windows-2022-x86_64.zip",
            sha256: "0beaf8ee4765b10f1d12d0ee0c872209935fa48184424842aa6fd299a6e3f5a8",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyruntime-nodejs-0.2.2-windows-11-arm64.zip",
            sha256: "d47571781dc17edfb0438943fbe2026417d33414904667ade0f9cb6de27e5733",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinyruntime-python` module: the Python half of the router's knowledge.
///
/// Answers which host interpreters count, which standalone build to install, and
/// what a warm Python worker is. It installs nothing itself.
///
/// Released alongside the router and pinned the same way — see [`TINYRUNTIME`].
const TINYRUNTIME_PYTHON: ModuleRecord = ModuleRecord {
    id: "tinyruntime-python",
    description: "Python runtime provider for tinyruntime",
    bus_name: "ai.tinyhumans.runtime.python.Provider",
    object_path: "/ai/tinyhumans/runtime/python/Provider",
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinyruntime-python/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyruntime-python-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8d020d8af32f2735e646e164124a84027d260638a1d3cfa392e7c97de179eca6",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyruntime-python-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "49fb3458636a8247b9735d80a573538bec8c73f8323e9ad0e2eaf5715b88edf1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyruntime-python-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "4f7e23f6f20df2820489f3cde4445e319c5b4c5285bb37e113112f7d83d37a57",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyruntime-python-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "89ca7864016bd62d2b247fc791b800acf7bbe8903bf40a12da2396e1396a9f63",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyruntime-python-0.2.2-macos-26-arm64.tar.gz",
            sha256: "2d091cbb29dc9d06996f290eaea8f03cf027e8fc9cff72824b9eae86d7ce5483",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyruntime-python-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "b0ec8c06202bf148463a087920387d3f243761756a570a334af16b9ba473267f",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyruntime-python-0.2.2-macos-15-arm64.tar.gz",
            sha256: "5577ed48e84d35ec07d0de8db29c840e0addcd5e54792a02b714e883a65a7ed8",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyruntime-python-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "e08fb6a06a47fd3a1e4e9ae1b6a52f42f3b78655c5f91f4e5dbd7448d6db19a4",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyruntime-python-0.2.2-windows-2025-x86_64.zip",
            sha256: "e22d5120ae58f9562a9861cd2c84a4d88ac692fa12d283ae047aafbe1a71adcc",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyruntime-python-0.2.2-windows-2022-x86_64.zip",
            sha256: "41f27a63ad1e5cc2559ed2fa11d698a775dad55763c7b5e5c884a3ef14f1a811",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyruntime-python-0.2.2-windows-11-arm64.zip",
            sha256: "0e96e8c0dbf1cfd497c8691928659c9f0bb3bf42a77eaa02bce59547f63b929e",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// Every module this build can load.
pub const ALL: &[ModuleRecord] = &[
    TINYDOCS,
    TINYWALLET,
    TINYMEMORY,
    TINYJUICE,
    TINYVOICE,
    TINYRUNTIME,
    TINYRUNTIME_NODEJS,
    TINYRUNTIME_PYTHON,
];

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

    /// Every host key `platform` can produce, across the supported triples.
    fn every_host_key() -> Vec<String> {
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
        let mut keys: Vec<String> = hosts
            .into_iter()
            .flat_map(|(os, arch, glibc)| candidates_for(os, arch, glibc))
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn a_record_that_pins_a_release_covers_every_host_the_platform_table_offers() {
        // The two tables are written independently and would drift silently:
        // `platform` offering a key no release publishes turns a supported host
        // into an "unsupported host" at first use.
        //
        // Scoped to records that pin a release at all. A record with no assets
        // is a module this build knows but has no published artifact for; it
        // loads from a developer build or the module search path, and asserting
        // release coverage for a release that does not exist would only assert
        // that it does not exist. The partial-coverage case — the one that is
        // actually a bug — is caught below.
        for record in ALL.iter().filter(|record| !record.assets.is_empty()) {
            for key in every_host_key() {
                assert!(
                    record.asset_for(&key).is_some(),
                    "{} publishes no asset for {key}, which the platform table would ask for",
                    record.id
                );
            }
        }
    }

    #[test]
    fn a_record_publishes_for_every_host_or_for_none() {
        // Partial coverage is the drift that hurts: it looks supported until a
        // user on the missing platform reaches the feature. All-or-nothing keeps
        // "not published yet" distinguishable from "published and incomplete".
        for record in ALL {
            let covered = every_host_key()
                .into_iter()
                .filter(|key| record.asset_for(key).is_some())
                .count();
            assert!(
                covered == 0 || covered == every_host_key().len(),
                "{} publishes assets for {covered} of {} host keys",
                record.id,
                every_host_key().len()
            );
        }
    }

    #[test]
    fn find_resolves_known_ids_only() {
        assert!(find("tinydocs").is_some());
        assert!(find("not-a-module").is_none());
    }
}
