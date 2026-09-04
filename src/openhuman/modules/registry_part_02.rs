
/// The `tinymcp` module: the Model Context Protocol client.
///
/// Owns both transports (Streamable HTTP and a subprocess over stdio), the
/// statically declared server set a host puts in its own configuration, the
/// dynamic registry of user-installed servers with its SQLite store, the
/// reconnect supervisor, the browser sign-in flow, and the write-audit log.
///
/// Lazy, because dialing an MCP server is something most sessions never do: a
/// host with no installed servers and no configured ones would otherwise pay a
/// download and a `dlopen` for a capability it never reaches. That differs from
/// the module's own `lazy = false` export hint, which speaks for a host whose
/// servers should be connected the moment it comes up — this host decides when
/// that moment is, and does so on the first ask.
///
/// **What stays out of the module is host policy**, and the split is the same
/// one the contract's own documentation draws: the prompt-injection scan over
/// remote tool definitions, the `mcp_clients` / `mcp_setup` RPC surface, the
/// agent-facing tools, and the proxy *scoping* decision all belong to this
/// application's threat model, not to a protocol client. `tinymcp-bus` carries
/// the vocabulary; this table says which bytes may speak it.
const TINYMCP: ModuleRecord = ModuleRecord {
    id: "tinymcp",
    description: "Model Context Protocol client: transports, registry, and the write-audit log",
    bus_name: "ai.tinyhumans.tinymcp.Mcp",
    object_path: "/ai/tinyhumans/tinymcp/Mcp",
    version: "0.3.2",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.3.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.3.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8bb03dcec777fbd52fedf678dafc04e44afeabc453b3459aace76e721bde7450",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.3.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "cdb06140a3d763c6137dc8470a6896f30707909bc6ac896088391fece220e284",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.3.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "879de1fb22e4b0b9383638ef00d207ed580a23c6b1fbc85a96b9d405c7e4273d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.3.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "324a448f1fd3b564f9c3892fe48f96415cd1c3a33f2c234e3c805410136fe7e2",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.3.2-macos-26-arm64.tar.gz",
            sha256: "dd952d4bdf865e9a8b5b358267f7f0c0895d15e9d657c5fc82f15f48f0b281eb",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.3.2-macos-26-x86_64.tar.gz",
            sha256: "11d284c1f9b194c5ac3865656e19b4d4ed3ca91a70e28359f14d13ac73101b1c",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.3.2-macos-15-arm64.tar.gz",
            sha256: "fc86f823719d305de6abc321d88a5b455517c4a6945135af15f7fbc2a3fca403",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.3.2-macos-15-x86_64.tar.gz",
            sha256: "7aec3ab842a7b2c6162d98021873416705b0da5c685ae0c0d8d3792684c0a530",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.3.2-windows-2025-x86_64.zip",
            sha256: "d0defc7df1f4bf4084ebaa1c373316e44f51d27f0ce32b35ac26937905fddda1",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.3.2-windows-2022-x86_64.zip",
            sha256: "71a35710fa45dc07c4f3d24074a189e5cd2ebff678276a57c7b25d907353fe3e",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.3.2-windows-11-arm64.zip",
            sha256: "cd31640774b27adf0472aaf0ad43f3a3c3e9d1b716624b40816c71240abdfa9e",
        },
    ],
    load: LoadPolicy::Lazy,
};

const TINYCONNECTORS: ModuleRecord = ModuleRecord {
    id: "tinyconnectors",
    description: "OAuth connector integrations: accounts, actions, triggers, and record sync",
    bus_name: "ai.tinyhumans.connectors.Composio",
    object_path: "/ai/tinyhumans/connectors/Composio",
    version: "0.8.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.8.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.8.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3cdd2c4b119b2da3ce0082bada68eeff89ee8554b2f1959065ca97d0eb6f1596",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.8.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "f2e63c9042ea75e11134b06a9ddff0a5f5d8edf24a6a300a723957cd5765c027",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.8.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "a29759a86d76b788ca58ff1a24f47de54a4fe82402e3c51161ed333a63932231",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.8.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "0cd3fd23cbf62a0ba9f4932c7d94f4cf3553e6d137b2ba3600c234486b0c18d5",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.8.0-macos-26-arm64.tar.gz",
            sha256: "e9a97f70620b811ee63d458f0f72f06eb57cc3efa94bbd656088a2f961555f6a",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.8.0-macos-26-x86_64.tar.gz",
            sha256: "e27f2ac2f34f943dc3d3ae9fdfd3ae8b1742e94569bad6cdbfce54a52186c1d9",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.8.0-macos-15-arm64.tar.gz",
            sha256: "19c6fbc6dc5b9504424b35dd8600be7d80071bbeb5172b99588edce74d45ef5f",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.8.0-macos-15-x86_64.tar.gz",
            sha256: "d6022e8d32834162ed269230eb50ea1e7263b6dd41d333b83f017eee17cc0bc3",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.8.0-windows-2025-x86_64.zip",
            sha256: "ace5366027828436ae2d3d78fe45dc4907fd4385fcc7b351e6f0cceb9dc58cdd",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.8.0-windows-2022-x86_64.zip",
            sha256: "a2db1ca277c2287d26fcb0cb99ad2e2c5000cbe3992adf801dc33de7ec78d291",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.8.0-windows-11-arm64.zip",
            sha256: "f10a348ec43beea290ec42a836a49575cdcb970b7355712286464e90fa40f9c4",
        },
    ],
    // Lazy: a user with no connected accounts should not pay to load it, and
    // most sessions never touch a connector. Safe even signed out — the module
    // loads without configuration and still answers the capability members.
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
    TINYMCP,
    TINYCONNECTORS,
];

/// The record for `id`, if this build knows it.
#[must_use]
pub fn find(id: &str) -> Option<&'static ModuleRecord> {
    ALL.iter().find(|record| record.id == id)
}
