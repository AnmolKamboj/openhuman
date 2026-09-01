//! One-shot memory doctor over a real workspace, for operator use:
//! OPENHUMAN_WORKSPACE=<workspace> cargo run --example run_memory_doctor

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let mut config = openhuman_core::openhuman::config::Config::load_or_init()
        .await
        .unwrap_or_default();
    config.apply_env_overrides();

    openhuman_core::openhuman::memory::host::install_memory_event_sink();
    #[cfg(feature = "modules")]
    openhuman_core::openhuman::modules::memory::set_modules_policy(std::sync::Arc::new(
        config.clone(),
    ));

    let report = openhuman_core::openhuman::memory::tree::health::report::run_doctor(&config).await;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
