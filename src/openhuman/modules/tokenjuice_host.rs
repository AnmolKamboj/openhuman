//! Host-owned optional ML callback served to the TinyJuice module.

use tinybus::ObjectPath;

const NAME: &str = "ai.tinyhumans.tinyjuice.MlHost";
const PATH: &str = "/ai/tinyhumans/tinyjuice/MlHost";

#[derive(Clone)]
struct MlHost;

#[tinybus::interface(name = "ai.tinyhumans.tinyjuice.MlHost")]
impl MlHost {
    async fn compress(
        &self,
        text: String,
        options: serde_json::Value,
    ) -> tinybus::Result<Option<String>> {
        let options = serde_json::from_value(options).map_err(method_error)?;
        crate::openhuman::inference::tokenjuice::ml::compress(&text, &options)
            .await
            .map_err(method_error)
    }
}

fn method_error(error: impl std::fmt::Display) -> tinybus::Error {
    tinybus::Error::MethodFailed {
        name: "ai.tinyhumans.tinyjuice.Error.Host".to_string(),
        message: error.to_string(),
    }
}

pub(super) async fn install(connection: &tinybus::Connection) -> tinybus::Result<()> {
    connection.serve_at(ObjectPath::new(PATH)?, MlHost).await?;
    connection.request_name(NAME).await
}
