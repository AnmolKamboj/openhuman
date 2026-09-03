#[async_trait]
impl MemoryCore for ModuleMemoryProvider {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        taint: MemoryTaint,
    ) -> Result<(), MemoryError> {
        // No log line carries `namespace`, `key` or `content`: all three are user
        // memory content.
        self.proxy("store")
            .await?
            .call::<()>(
                methods::STORE,
                (
                    namespace,
                    key,
                    content,
                    category,
                    session_id.map(str::to_string),
                    taint,
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        self.proxy("get")
            .await?
            .call(methods::GET, (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, MemoryError> {
        self.proxy("forget")
            .await?
            .call(methods::FORGET, (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.proxy("list")
            .await?
            .call(
                methods::LIST,
                (
                    namespace.map(str::to_string),
                    category.cloned(),
                    session_id.map(str::to_string),
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn namespaces(&self) -> Result<Vec<NamespaceSummary>, MemoryError> {
        self.proxy("namespaces")
            .await?
            .call(methods::NAMESPACES, ())
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryRecall for ModuleMemoryProvider {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &OwnedRecallOpts,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // `scope` crosses as a value because the driver must apply it as a query
        // predicate internally; narrowing the result here instead would let the
        // module spend its `limit` on entries the caller may not see.
        self.proxy("recall")
            .await?
            .call(methods::RECALL, (query, limit, opts, scope.cloned()))
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryPortability for ModuleMemoryProvider {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ExportPage, MemoryError> {
        self.proxy("export_page")
            .await?
            .call(methods::EXPORT_PAGE, (cursor.map(str::to_string), limit))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn import_records(
        &self,
        records: Vec<ExportRecord>,
    ) -> Result<ImportOutcome, MemoryError> {
        self.proxy("import_records")
            .await?
            .call(methods::IMPORT_RECORDS, (records,))
            .await
            .map_err(|error| from_bus(&error))
    }
}
#[async_trait]
impl MemoryIngest for ModuleMemoryProvider {
    async fn ingest_document(&self, item: IngestItem) -> Result<IngestOutcome, MemoryError> {
        module_call!(self, "ingest_document", methods::INGEST_DOCUMENT, (item,))
    }
    async fn ingest_chat(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        module_call_slow!(self, "ingest_chat", methods::INGEST_CHAT, (messages,))
    }
    async fn ingest_email(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        module_call_slow!(self, "ingest_email", methods::INGEST_EMAIL, (messages,))
    }
}
