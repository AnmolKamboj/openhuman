
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
    version: "0.3.1",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.3.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.3.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "f2ba8bfa0a74a9c234499e946936cc2de7f237e9772a85e7df0453a7c29669ab",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.3.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "a68734086449b980a7de3cf87f6b6e00f4aa43bbd1f39187ac0803b4082d62dc",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.3.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "da9225bbc008a3de0917da0280667e4a76a93d517f017a0962f420a0c0b311f6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.3.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "aaefc1f2c3ae51bb1447d18b25abc5b78f37b795c172ecb6d8fa31967b8d213c",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.3.1-macos-26-arm64.tar.gz",
            sha256: "453624140df1d0df00a6e1bb1108fab086fb6b2ed3fbdced3f6765b534d4e0bc",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.3.1-macos-26-x86_64.tar.gz",
            sha256: "bcb8c68b0744bfc82479261a60c1beb3fcc0842321bcd5ed6519aef1b6194ac6",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.3.1-macos-15-arm64.tar.gz",
            sha256: "a3e405131221a168bf35a887e1628f66e5e27a2cae65db8c260b00b4e190f4bb",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.3.1-macos-15-x86_64.tar.gz",
            sha256: "6e501853e76ba77b7fea2f828f3ff293c15ad3e41a5b70b7f3ddb73c6efee5d1",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.3.1-windows-2025-x86_64.zip",
            sha256: "86d2d309e8c605c27f87bd652709683430449878843beb684842965e3fba2a41",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.3.1-windows-2022-x86_64.zip",
            sha256: "7b04601dafd43eddf7d218900a1e70ecfa6471aa3af34f4dcab67fce2d872a99",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.3.1-windows-11-arm64.zip",
            sha256: "db352cb7fffdbbd00a5cbd3e4fc610123e23c884c2d0cad89f1293a83f550406",
        },
    ],
    load: LoadPolicy::Lazy,
};

const TINYCONNECTORS: ModuleRecord = ModuleRecord {
    id: "tinyconnectors",
    description: "OAuth connector integrations: accounts, actions, triggers, and record sync",
    bus_name: "ai.tinyhumans.connectors.Composio",
    object_path: "/ai/tinyhumans/connectors/Composio",
    version: "0.7.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.7.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.7.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "881f31eecc1d91ace45a5211669ce0b575ead6a1cb86ca1a6d5608c9304a45c8",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.7.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "1784e73ba56228772229035fc5f32ce0c4e012057e5fdd079ebf9ee5337d6229",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.7.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "b73df16260dce3387f7de915dfa93f6c0f8c977cf21784b97b2a6b2dcdf961f7",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.7.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "4e09ba65e7bd6cec8a3ddbcaabac01e156bacfd4aa83edca30e59bc4973eee8d",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.7.0-macos-26-arm64.tar.gz",
            sha256: "8b6dacbf22f32aa73eef512a6a5e50ca5def19a3d6ef948a2f3599b0ebe676e3",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.7.0-macos-26-x86_64.tar.gz",
            sha256: "fc0fcab43797fb22fadba83af901b1fc5621b4b7486dcad5a6db474842d36396",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.7.0-macos-15-arm64.tar.gz",
            sha256: "45cab192ae57a3a1ab2550d41740986f74f1e2e582f3b336d0a6dbf35e10cfcb",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.7.0-macos-15-x86_64.tar.gz",
            sha256: "d4a8b5e6833b56ad91bf1b2140e8cc04a38f1c9922e2fd2c08112d6b9f0c867d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.7.0-windows-2025-x86_64.zip",
            sha256: "3ce794911a57d0f539fb63c4e0c7c3a3cd9775760e329af0146a2c3aff64a055",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.7.0-windows-2022-x86_64.zip",
            sha256: "a84c91732e432ef0ac7ffe81b639ddd4cdbbcad4d61e08694623d5201c8ee62f",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.7.0-windows-11-arm64.zip",
            sha256: "7034918df432aecfa1f402a5640ce5aff2ca051d8ced8f5622a0ce519f300284",
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
