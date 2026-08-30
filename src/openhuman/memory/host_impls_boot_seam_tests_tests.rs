use super::*;

/// Installing the seams must satisfy the engine's `require_embedding_host`.
///
/// This is the regression for the outage #5560 shipped and had to take back:
/// the seam install was removed from all three boot sites on the reasoning
/// that "this process embeds no engine, so there is nothing to call back".
/// It does embed one — `tinymemory-core` is a normal dependency, and
/// `session::builder::factory` reaches `store::factories::
/// create_session_memory_with_local_ai`, which calls
/// `require_embedding_host()`. Every chat turn then died with
///
///   no EmbeddingHost installed — the host must call
///   memory::embedding_host::set_embedding_host during startup wiring
///
/// The loaded module installing its own seams does not cover this: a
/// `cdylib` has its own statics, so what it sets is invisible in this
/// process. Nothing in a build or a type check said so, which is why the
/// assertion is here.
///
/// It asserts the engine's own accessor rather than a local flag, so it
/// keeps testing the thing the engine actually reads.
#[test]
fn installing_the_seams_satisfies_the_engines_embedding_host() {
    install_for_tests();

    assert!(
        tinymemory_core::embedding_host::embedding_host().is_some(),
        "boot installed no EmbeddingHost; the session memory factory on the \
         chat path calls require_embedding_host() and will fail every turn"
    );
    assert!(
        tinymemory_core::embedding_host::require_embedding_host().is_ok(),
        "require_embedding_host must succeed once the boot seams are in"
    );
}
