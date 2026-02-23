import { useMemo, useState } from 'react';
import {
  CogIcon,
  CpuChipIcon,
  ShieldCheckIcon,
  ServerIcon,
  WrenchScrewdriverIcon,
  ChatBubbleLeftRightIcon,
  DocumentTextIcon,
} from '@heroicons/react/24/outline';

import SettingsHeader from '../components/SettingsHeader';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';
import SectionCard from './components/SectionCard';
import InputGroup, { Field, CheckboxField } from './components/InputGroup';
import ActionPanel, { PrimaryButton } from './components/ActionPanel';
import {
  alphahumanAgentChat,
  alphahumanDecryptSecret,
  alphahumanDoctorModels,
  alphahumanDoctorReport,
  alphahumanGetIntegrationInfo,
  alphahumanGetConfig,
  alphahumanHardwareDiscover,
  alphahumanHardwareIntrospect,
  alphahumanListIntegrations,
  alphahumanMigrateOpenclaw,
  alphahumanModelsRefresh,
  alphahumanUpdateGatewaySettings,
  alphahumanUpdateMemorySettings,
  alphahumanUpdateModelSettings,
  alphahumanUpdateRuntimeSettings,
  alphahumanUpdateTunnelSettings,
  alphahumanServiceInstall,
  alphahumanServiceStart,
  alphahumanServiceStatus,
  alphahumanServiceStop,
  alphahumanServiceUninstall,
  alphahumanEncryptSecret,
  isTauri,
  TunnelConfig,
  runtimeDisableSkill,
  runtimeEnableSkill,
  runtimeIsSkillEnabled,
  runtimeListSkills,
  SkillSnapshot,
} from '../../../utils/tauriCommands';

const formatJson = (value: unknown) => JSON.stringify(value, null, 2);

const TauriCommandsPanel = () => {
  const { navigateBack } = useSettingsNavigation();

  // View mode removed - always show all sections
  const [expandedSections] = useState<Set<string>>(
    new Set(['system-configuration', 'runtime-execution', 'security-data', 'network-infrastructure', 'development-operations', 'interactive-tools'])
  );

  // Output and error states
  const [output, setOutput] = useState<string>('');
  const [error, setError] = useState<string>('');

  // Form states (preserved from original)
  const [providerOverride, setProviderOverride] = useState<string>('');
  const [integrationName, setIntegrationName] = useState<string>('');
  const [hardwarePath, setHardwarePath] = useState<string>('');
  const [migrationSource, setMigrationSource] = useState<string>('');
  const [encryptInput, setEncryptInput] = useState<string>('');
  const [decryptInput, setDecryptInput] = useState<string>('');
  const [apiKey, setApiKey] = useState<string>('');
  const [apiUrl, setApiUrl] = useState<string>('');
  const [defaultProvider, setDefaultProvider] = useState<string>('');
  const [defaultModel, setDefaultModel] = useState<string>('');
  const [defaultTemp, setDefaultTemp] = useState<string>('0.7');
  const [tunnelProvider, setTunnelProvider] = useState<string>('none');
  const [cloudflareToken, setCloudflareToken] = useState<string>('');
  const [ngrokToken, setNgrokToken] = useState<string>('');
  const [tailscaleHostname, setTailscaleHostname] = useState<string>('');
  const [customCommand, setCustomCommand] = useState<string>('');
  const [gatewayHost, setGatewayHost] = useState<string>('127.0.0.1');
  const [gatewayPort, setGatewayPort] = useState<string>('3000');
  const [gatewayPairing, setGatewayPairing] = useState<boolean>(true);
  const [gatewayPublic, setGatewayPublic] = useState<boolean>(false);
  const [memoryBackend, setMemoryBackend] = useState<string>('sqlite');
  const [memoryAutoSave, setMemoryAutoSave] = useState<boolean>(true);
  const [embeddingProvider, setEmbeddingProvider] = useState<string>('none');
  const [embeddingModel, setEmbeddingModel] = useState<string>('text-embedding-3-small');
  const [embeddingDims, setEmbeddingDims] = useState<string>('1536');
  const [runtimeKind, setRuntimeKind] = useState<string>('native');
  const [reasoningEnabled, setReasoningEnabled] = useState<boolean>(false);
  const [skills, setSkills] = useState<
    Array<{ snapshot: SkillSnapshot; enabled: boolean }>
  >([]);
  const [skillsLoading, setSkillsLoading] = useState<boolean>(false);
  const [chatInput, setChatInput] = useState<string>('');
  const [chatProvider, setChatProvider] = useState<string>('');
  const [chatModel, setChatModel] = useState<string>('');
  const [chatTemperature, setChatTemperature] = useState<string>('0.7');
  const [chatLog, setChatLog] = useState<Array<{ role: 'user' | 'agent'; text: string }>>([]);

  // Loading states
  const [operationLoading, setOperationLoading] = useState<string>('');

  const tauriAvailable = useMemo(() => isTauri(), []);
  const parseOptionalNumber = (value: string): number | null => {
    if (!value.trim()) {
      return null;
    }
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  };

  const run = async (fn: () => Promise<unknown>, operationName?: string) => {
    setError('');
    if (operationName) setOperationLoading(operationName);
    try {
      const result = await fn();
      setOutput(formatJson(result));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setOperationLoading('');
    }
  };

  const runWithResult = async <T,>(fn: () => Promise<T>, operationName?: string): Promise<T | null> => {
    setError('');
    if (operationName) setOperationLoading(operationName);
    try {
      const result = await fn();
      setOutput(formatJson(result));
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      return null;
    } finally {
      setOperationLoading('');
    }
  };

  const loadConfig = async () => {
    const response = await runWithResult(() => alphahumanGetConfig(), 'loadConfig');
    if (!response) {
      return;
    }
    try {
      const snapshot = response.result;
      const config = snapshot.config as Record<string, unknown>;
      setApiKey((config.api_key as string) ?? '');
      setApiUrl((config.api_url as string) ?? '');
      setDefaultProvider((config.default_provider as string) ?? '');
      setDefaultModel((config.default_model as string) ?? '');
      setDefaultTemp(String((config.default_temperature as number) ?? 0.7));

      const tunnel = (config.tunnel as Record<string, unknown>) ?? {};
      setTunnelProvider((tunnel.provider as string) ?? 'none');
      setCloudflareToken(((tunnel.cloudflare as Record<string, unknown>)?.token as string) ?? '');
      setNgrokToken(((tunnel.ngrok as Record<string, unknown>)?.auth_token as string) ?? '');
      setTailscaleHostname(((tunnel.tailscale as Record<string, unknown>)?.hostname as string) ?? '');
      setCustomCommand(((tunnel.custom as Record<string, unknown>)?.start_command as string) ?? '');

      const gateway = (config.gateway as Record<string, unknown>) ?? {};
      setGatewayHost((gateway.host as string) ?? '127.0.0.1');
      setGatewayPort(String((gateway.port as number) ?? 3000));
      setGatewayPairing((gateway.require_pairing as boolean) ?? true);
      setGatewayPublic((gateway.allow_public_bind as boolean) ?? false);

      const memory = (config.memory as Record<string, unknown>) ?? {};
      setMemoryBackend((memory.backend as string) ?? 'sqlite');
      setMemoryAutoSave((memory.auto_save as boolean) ?? true);
      setEmbeddingProvider((memory.embedding_provider as string) ?? 'none');
      setEmbeddingModel((memory.embedding_model as string) ?? 'text-embedding-3-small');
      setEmbeddingDims(String((memory.embedding_dimensions as number) ?? 1536));

      const runtime = (config.runtime as Record<string, unknown>) ?? {};
      setRuntimeKind((runtime.kind as string) ?? 'native');
      setReasoningEnabled((runtime.reasoning_enabled as boolean) ?? false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    }
  };

  const buildTunnelConfig = (): TunnelConfig => {
    if (tunnelProvider === 'cloudflare') {
      return {
        provider: 'cloudflare',
        cloudflare: { token: cloudflareToken },
      };
    }
    if (tunnelProvider === 'ngrok') {
      return {
        provider: 'ngrok',
        ngrok: { auth_token: ngrokToken },
      };
    }
    if (tunnelProvider === 'tailscale') {
      return {
        provider: 'tailscale',
        tailscale: { hostname: tailscaleHostname || null },
      };
    }
    if (tunnelProvider === 'custom') {
      return {
        provider: 'custom',
        custom: { start_command: customCommand },
      };
    }
    return { provider: 'none' };
  };

  const saveModelSettings = () =>
    run(() =>
      alphahumanUpdateModelSettings({
        api_key: apiKey.trim() ? apiKey : null,
        api_url: apiUrl.trim() ? apiUrl : null,
        default_provider: defaultProvider.trim() ? defaultProvider : null,
        default_model: defaultModel.trim() ? defaultModel : null,
        default_temperature: parseOptionalNumber(defaultTemp),
      }),
      'saveModelSettings'
    );

  const saveTunnelSettings = () => run(() => alphahumanUpdateTunnelSettings(buildTunnelConfig()), 'saveTunnelSettings');

  const saveGatewaySettings = () =>
    run(() =>
      alphahumanUpdateGatewaySettings({
        host: gatewayHost.trim() ? gatewayHost : null,
        port: parseOptionalNumber(gatewayPort),
        require_pairing: gatewayPairing,
        allow_public_bind: gatewayPublic,
      }),
      'saveGatewaySettings'
    );

  const saveMemorySettings = () =>
    run(() =>
      alphahumanUpdateMemorySettings({
        backend: memoryBackend.trim() ? memoryBackend : null,
        auto_save: memoryAutoSave,
        embedding_provider: embeddingProvider.trim() ? embeddingProvider : null,
        embedding_model: embeddingModel.trim() ? embeddingModel : null,
        embedding_dimensions: parseOptionalNumber(embeddingDims),
      }),
      'saveMemorySettings'
    );

  const saveRuntimeSettings = () =>
    run(() =>
      alphahumanUpdateRuntimeSettings({
        kind: runtimeKind.trim() ? runtimeKind : null,
        reasoning_enabled: reasoningEnabled,
      }),
      'saveRuntimeSettings'
    );

  const loadSkills = async () => {
    setSkillsLoading(true);
    try {
      const snapshots = await runtimeListSkills();
      const enriched = await Promise.all(
        snapshots.map(async (snapshot) => ({
          snapshot,
          enabled: await runtimeIsSkillEnabled(snapshot.skill_id),
        }))
      );
      setSkills(enriched);
      setOutput(formatJson(enriched));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setSkillsLoading(false);
    }
  };

  const toggleSkill = async (skillId: string, nextEnabled: boolean) => {
    if (nextEnabled) {
      await run(() => runtimeEnableSkill(skillId), 'enableSkill');
    } else {
      await run(() => runtimeDisableSkill(skillId), 'disableSkill');
    }
    setSkills((prev) =>
      prev.map((item) =>
        item.snapshot.skill_id === skillId
          ? { ...item, enabled: nextEnabled }
          : item
      )
    );
  };

  const sendChat = async () => {
    if (!chatInput.trim()) {
      return;
    }
    const userMessage = chatInput.trim();
    setChatLog((prev) => [...prev, { role: 'user', text: userMessage }]);
    setChatInput('');
    const response = await runWithResult(() =>
      alphahumanAgentChat(
        userMessage,
        chatProvider.trim() ? chatProvider : undefined,
        chatModel.trim() ? chatModel : undefined,
        parseOptionalNumber(chatTemperature) ?? undefined
      ),
      'sendChat'
    );
    if (response) {
      setChatLog((prev) => [...prev, { role: 'agent', text: response.result }]);
    }
  };

  // Always show all sections
  const isSectionVisible = () => {
    return true; // Always show all sections
  };

  const isCollapsed = (sectionId: string) => {
    return !expandedSections.has(sectionId);
  };

  // Helper to check if a section is collapsed (currently unused but kept for future expansion)
  // const toggleSection = (sectionId: string) => {
  //   const newExpanded = new Set(expandedSections);
  //   if (newExpanded.has(sectionId)) {
  //     newExpanded.delete(sectionId);
  //   } else {
  //     newExpanded.add(sectionId);
  //   }
  //   setExpandedSections(newExpanded);
  // };

  return (
    <div className="h-full flex flex-col">
      <SettingsHeader
        title="Tauri Command Console"
        showBackButton={true}
        onBack={navigateBack}
      />

      <div className="flex-1 overflow-y-auto px-6 pb-12 space-y-10">
        {!tauriAvailable && (
          <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-200">
            Tauri runtime not detected. Commands will fail in browser mode.
          </div>
        )}

        {operationLoading && (
          <div className="flex items-center justify-end">
            <div className="flex items-center gap-2 text-sm text-gray-400">
              <div className="h-4 w-4 border-2 border-white/20 border-t-white rounded-full animate-spin" />
              {operationLoading}
            </div>
          </div>
        )}

        {/* Critical Path - Always visible in grid on desktop */}
        <div className="grid gap-8 lg:grid-cols-2">
          {/* Category 1: System Configuration */}
          {isSectionVisible() && (
            <SectionCard
              title="System Configuration"
              priority="infrastructure"
              icon={<CogIcon />}
              collapsible={true}
              defaultExpanded={!isCollapsed('system-configuration')}
              hasChanges={false}
              loading={operationLoading === 'loadConfig' || operationLoading === 'saveModelSettings'}
          >
            <InputGroup
              title="Model API Keys"
              description="Configure your AI model provider settings"
            >
              <Field
                label="API Key"
                helpText="Enter your AI provider's API key for authentication. This enables the agent to access language models for conversation and reasoning. Keep this secure and never share it publicly."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="sk-..."
                  value={apiKey}
                  onChange={(event) => setApiKey(event.target.value)}
                />
              </Field>
              <Field
                label="API Base URL"
                helpText="Custom API endpoint for your AI provider. Default URLs work for standard providers. Change only when using custom deployments, proxy servers, or alternative API-compatible services."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="https://api.openai.com/v1"
                  value={apiUrl}
                  onChange={(event) => setApiUrl(event.target.value)}
                />
              </Field>
              <Field
                label="Default Provider"
                helpText="Primary AI provider for agent operations. Supported providers include 'openai', 'anthropic', 'azure', 'cohere'. This determines which AI service processes your conversations and reasoning tasks."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="openai"
                  value={defaultProvider}
                  onChange={(event) => setDefaultProvider(event.target.value)}
                />
              </Field>
              <Field
                label="Default Model"
                helpText="Primary language model for agent interactions. GPT-4 offers advanced reasoning but higher costs, GPT-3.5-turbo is faster and economical. Claude models excel at analysis and safety."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="gpt-4.1-mini"
                  value={defaultModel}
                  onChange={(event) => setDefaultModel(event.target.value)}
                />
              </Field>
              <Field
                label="Temperature"
                helpText="Controls randomness in AI responses (0.0-2.0). Lower values (0.1-0.3) for factual tasks, medium (0.5-0.8) for balanced responses, higher (0.8-1.5) for creative tasks. Default 0.7 works well for most conversations."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="0.7"
                  value={defaultTemp}
                  onChange={(event) => setDefaultTemp(event.target.value)}
                />
              </Field>
            </InputGroup>

            <ActionPanel>
              <PrimaryButton
                onClick={loadConfig}
                loading={operationLoading === 'loadConfig'}
              >
                Load Config
              </PrimaryButton>
              <PrimaryButton
                onClick={saveModelSettings}
                loading={operationLoading === 'saveModelSettings'}
              >
                Save Model Settings
              </PrimaryButton>
            </ActionPanel>
          </SectionCard>
          )}

          {/* Category 2: Runtime & Execution */}
          {isSectionVisible() && (
            <SectionCard
              title="Runtime & Execution"
              priority="infrastructure"
              icon={<CpuChipIcon />}
              collapsible={true}
              defaultExpanded={!isCollapsed('runtime-execution')}
              hasChanges={false}
              loading={operationLoading === 'saveRuntimeSettings' || skillsLoading}
          >
            <InputGroup
              title="Runtime Settings"
              description="Configure V8 runtime and skill execution"
            >
              <Field
                label="Runtime Kind"
                helpText="JavaScript execution environment for skills. 'native' uses V8 engine for maximum performance and compatibility. 'docker' provides isolation but requires Docker. 'wasm' offers security with some limitations."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="native"
                  value={runtimeKind}
                  onChange={(event) => setRuntimeKind(event.target.value)}
                />
              </Field>
              <CheckboxField
                label="Reasoning enabled"
                helpText="Activates advanced step-by-step reasoning capabilities in AI responses. Improves accuracy for complex tasks but increases response time and token usage. Recommended for analytical and problem-solving tasks."
                checked={reasoningEnabled}
                onChange={setReasoningEnabled}
              />
            </InputGroup>

            <ActionPanel>
              <PrimaryButton
                onClick={saveRuntimeSettings}
                loading={operationLoading === 'saveRuntimeSettings'}
              >
                Save Runtime Settings
              </PrimaryButton>
              <PrimaryButton
                onClick={loadSkills}
                loading={skillsLoading}
                variant="outline"
              >
                {skillsLoading ? 'Loading Skills…' : 'Load Skills'}
              </PrimaryButton>
            </ActionPanel>

            {skills.length > 0 && (
              <div className="space-y-4">
                <h5 className="text-sm font-medium text-gray-300">Skills</h5>
                <div className="grid gap-3 max-h-52 overflow-y-auto">
                  {skills.map((item) => (
                    <div
                      key={item.snapshot.skill_id}
                      className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 backdrop-blur-sm px-4 py-3"
                    >
                      <div className="flex-1 min-w-0">
                        <div className="text-sm text-white font-medium">{item.snapshot.name}</div>
                        <div className="text-xs text-gray-400 truncate">
                          {item.snapshot.skill_id}
                        </div>
                      </div>
                      <CheckboxField
                        label={item.enabled ? 'Active' : 'Inactive'}
                        checked={item.enabled}
                        onChange={(checked) =>
                          toggleSkill(item.snapshot.skill_id, checked)
                        }
                        className="text-xs ml-4 flex-shrink-0"
                      />
                    </div>
                  ))}
                </div>
              </div>
            )}

            <InputGroup title="Service Management">
              <div className="md:col-span-2">
                <ActionPanel>
                  <PrimaryButton
                    onClick={() => run(alphahumanServiceStatus, 'serviceStatus')}
                    loading={operationLoading === 'serviceStatus'}
                  >
                    Status
                  </PrimaryButton>
                  <PrimaryButton
                    onClick={() => run(alphahumanServiceInstall, 'serviceInstall')}
                    loading={operationLoading === 'serviceInstall'}
                    variant="outline"
                  >
                    Install
                  </PrimaryButton>
                  <PrimaryButton
                    onClick={() => run(alphahumanServiceStart, 'serviceStart')}
                    loading={operationLoading === 'serviceStart'}
                    variant="outline"
                  >
                    Start
                  </PrimaryButton>
                  <PrimaryButton
                    onClick={() => run(alphahumanServiceStop, 'serviceStop')}
                    loading={operationLoading === 'serviceStop'}
                    variant="outline"
                  >
                    Stop
                  </PrimaryButton>
                  <PrimaryButton
                    onClick={() => run(alphahumanServiceUninstall, 'serviceUninstall')}
                    loading={operationLoading === 'serviceUninstall'}
                    variant="outline"
                  >
                    Uninstall
                  </PrimaryButton>
                </ActionPanel>
              </div>
            </InputGroup>
          </SectionCard>
          )}
        </div>

        {/* Category 3: Security & Data - Full width in basic mode, grid in advanced+ */}
        {isSectionVisible() && (
          <SectionCard
            title="Security & Data"
            priority="infrastructure"
            icon={<ShieldCheckIcon />}
            collapsible={true}
            defaultExpanded={!isCollapsed('security-data')}
            hasChanges={false}
            loading={operationLoading?.includes('Secret') || operationLoading?.includes('Models') || operationLoading?.includes('Integration')}
        >
          <div className="grid gap-6 lg:grid-cols-2">
            <InputGroup
              title="Secrets Management"
              description="Encrypt and decrypt sensitive data"
            >
              <Field
                label="Encrypt"
                helpText="Convert sensitive data to encrypted format using the system's secure encryption. Useful for safely storing API keys, tokens, or other confidential information in configuration files."
                fullWidth
              >
                <textarea
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200 min-h-[90px] resize-y"
                  placeholder="Plaintext"
                  value={encryptInput}
                  onChange={(event) => setEncryptInput(event.target.value)}
                />
              </Field>
              <Field
                label="Decrypt"
                helpText="Convert encrypted data back to readable format. Only works with data encrypted by this system. Use this to verify encrypted values or retrieve original content when needed."
                fullWidth
              >
                <textarea
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200 min-h-[90px] resize-y"
                  placeholder="Ciphertext"
                  value={decryptInput}
                  onChange={(event) => setDecryptInput(event.target.value)}
                />
              </Field>
            </InputGroup>

            <div className="space-y-6">
              <InputGroup title="Models">
                <Field
                  label="Provider Override"
                  helpText="Temporarily override the default AI provider for model operations. Leave empty to use the configured default provider. Useful for testing different providers or accessing provider-specific models."
                  fullWidth
                >
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="Provider override (optional)"
                    value={providerOverride}
                    onChange={(event) => setProviderOverride(event.target.value)}
                  />
                </Field>
              </InputGroup>

              <InputGroup title="Integrations">
                <Field
                  label="Integration Name"
                  helpText="Name of the platform integration to query or manage. Examples: 'telegram', 'gmail', 'discord'. Use this to get detailed information about specific platform connections and their status."
                  fullWidth
                >
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="Integration name"
                    value={integrationName}
                    onChange={(event) => setIntegrationName(event.target.value)}
                  />
                </Field>
              </InputGroup>
            </div>
          </div>

          <ActionPanel>
            <PrimaryButton
              onClick={() => run(() => alphahumanEncryptSecret(encryptInput), 'encryptSecret')}
              loading={operationLoading === 'encryptSecret'}
              disabled={!encryptInput.trim()}
            >
              Encrypt
            </PrimaryButton>
            <PrimaryButton
              onClick={() => run(() => alphahumanDecryptSecret(decryptInput), 'decryptSecret')}
              loading={operationLoading === 'decryptSecret'}
              disabled={!decryptInput.trim()}
              variant="outline"
            >
              Decrypt
            </PrimaryButton>
            <PrimaryButton
              onClick={() => run(() => alphahumanModelsRefresh(providerOverride || undefined, false), 'modelsRefresh')}
              loading={operationLoading === 'modelsRefresh'}
            >
              Refresh Models
            </PrimaryButton>
            <PrimaryButton
              onClick={() => run(() => alphahumanModelsRefresh(providerOverride || undefined, true), 'modelsForceRefresh')}
              loading={operationLoading === 'modelsForceRefresh'}
              variant="outline"
            >
              Force Refresh
            </PrimaryButton>
            <PrimaryButton
              onClick={() => run(alphahumanListIntegrations, 'listIntegrations')}
              loading={operationLoading === 'listIntegrations'}
            >
              List Integrations
            </PrimaryButton>
            <PrimaryButton
              onClick={() => run(() => alphahumanGetIntegrationInfo(integrationName), 'getIntegrationInfo')}
              loading={operationLoading === 'getIntegrationInfo'}
              disabled={!integrationName.trim()}
              variant="outline"
            >
              Get Integration Info
            </PrimaryButton>
          </ActionPanel>
        </SectionCard>
        )}

        {/* Category 4: Network & Infrastructure */}
        {isSectionVisible() && (
          <SectionCard
            title="Network & Infrastructure"
            priority="infrastructure"
            icon={<ServerIcon />}
            collapsible={true}
            defaultExpanded={!isCollapsed('network-infrastructure')}
            hasChanges={false}
            loading={operationLoading?.includes('Gateway') || operationLoading?.includes('Tunnel') || operationLoading?.includes('Memory')}
          >
            <div className="grid gap-8 lg:grid-cols-2">
              <InputGroup
                title="Gateway Settings"
                description="Configure API gateway host and port"
              >
                <Field
                  label="Host"
                  helpText="IP address for the API gateway server. Use '127.0.0.1' for local-only access or '0.0.0.0' to accept connections from any network interface. Change with caution for security."
                >
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="127.0.0.1"
                    value={gatewayHost}
                    onChange={(event) => setGatewayHost(event.target.value)}
                  />
                </Field>
                <Field
                  label="Port"
                  helpText="Network port for the agent's API server. Default 3000 works for most setups. Change if this port conflicts with other services. Ports below 1024 may require administrator privileges."
                >
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="3000"
                    value={gatewayPort}
                    onChange={(event) => setGatewayPort(event.target.value)}
                  />
                </Field>
                <CheckboxField
                  label="Require pairing"
                  helpText="Enforces device pairing before API access. Provides additional security by requiring explicit authorization. Disable only for development or trusted network environments."
                  checked={gatewayPairing}
                  onChange={setGatewayPairing}
                />
                <CheckboxField
                  label="Allow public bind"
                  helpText="Permits connections from external networks when enabled. WARNING: Only enable in secure environments with proper firewall protection. Disabled by default for security."
                  checked={gatewayPublic}
                  onChange={setGatewayPublic}
                />
              </InputGroup>

              <InputGroup
                title="Tunnel Configuration"
                description="Configure external tunnel providers"
              >
                <Field label="Provider" fullWidth>
                  <select
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    value={tunnelProvider}
                    onChange={(event) => setTunnelProvider(event.target.value)}
                  >
                    <option value="none">none</option>
                    <option value="cloudflare">cloudflare</option>
                    <option value="ngrok">ngrok</option>
                    <option value="tailscale">tailscale</option>
                    <option value="custom">custom</option>
                  </select>
                </Field>
                {tunnelProvider === 'cloudflare' && (
                  <Field label="Cloudflare Token" fullWidth>
                    <input
                      className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                      placeholder="cloudflare token"
                      value={cloudflareToken}
                      onChange={(event) => setCloudflareToken(event.target.value)}
                    />
                  </Field>
                )}
                {tunnelProvider === 'ngrok' && (
                  <Field label="Ngrok Token" fullWidth>
                    <input
                      className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                      placeholder="ngrok token"
                      value={ngrokToken}
                      onChange={(event) => setNgrokToken(event.target.value)}
                    />
                  </Field>
                )}
                {tunnelProvider === 'tailscale' && (
                  <Field label="Tailscale Hostname" fullWidth>
                    <input
                      className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                      placeholder="alpha.local"
                      value={tailscaleHostname}
                      onChange={(event) => setTailscaleHostname(event.target.value)}
                    />
                  </Field>
                )}
                {tunnelProvider === 'custom' && (
                  <Field label="Custom Start Command" fullWidth>
                    <input
                      className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                      placeholder="ngrok http 3000"
                      value={customCommand}
                      onChange={(event) => setCustomCommand(event.target.value)}
                    />
                  </Field>
                )}
              </InputGroup>
            </div>

            <InputGroup
              title="Memory Settings"
              description="Configure memory backend and embedding models"
            >
              <Field
                label="Backend"
                helpText="Memory storage system for conversations and agent memory. 'sqlite' for local file storage (default), 'postgres' for scalable database, 'redis' for high-performance caching, 'neo4j' for graph relationships."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="sqlite"
                  value={memoryBackend}
                  onChange={(event) => setMemoryBackend(event.target.value)}
                />
              </Field>
              <CheckboxField
                label="Auto-save"
                helpText="Automatically save conversation history and agent memory to the configured backend storage. Recommended for persistent memory across sessions and system restarts."
                checked={memoryAutoSave}
                onChange={setMemoryAutoSave}
              />
              <Field
                label="Embedding Provider"
                helpText="AI service for generating vector embeddings for semantic search and memory retrieval. 'openai' for high quality, 'cohere' for multilingual, 'huggingface' for local models, 'none' to disable."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="openai"
                  value={embeddingProvider}
                  onChange={(event) => setEmbeddingProvider(event.target.value)}
                />
              </Field>
              <Field
                label="Embedding Model"
                helpText="Specific model for generating vector embeddings. OpenAI: 'text-embedding-3-small' (fast, cost-effective) or 'text-embedding-3-large' (higher accuracy). Must match your provider."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="text-embedding-3-small"
                  value={embeddingModel}
                  onChange={(event) => setEmbeddingModel(event.target.value)}
                />
              </Field>
              <Field
                label="Embedding Dimensions"
                helpText="Vector size for embeddings. Must match your model: text-embedding-3-small supports 512-1536 (default 1536), text-embedding-3-large supports up to 3072. Higher dimensions = better accuracy, more storage."
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="1536"
                  value={embeddingDims}
                  onChange={(event) => setEmbeddingDims(event.target.value)}
                />
              </Field>
            </InputGroup>

            <ActionPanel>
              <PrimaryButton
                onClick={saveGatewaySettings}
                loading={operationLoading === 'saveGatewaySettings'}
              >
                Save Gateway Settings
              </PrimaryButton>
              <PrimaryButton
                onClick={saveTunnelSettings}
                loading={operationLoading === 'saveTunnelSettings'}
              >
                Save Tunnel Settings
              </PrimaryButton>
              <PrimaryButton
                onClick={saveMemorySettings}
                loading={operationLoading === 'saveMemorySettings'}
              >
                Save Memory Settings
              </PrimaryButton>
            </ActionPanel>
          </SectionCard>
        )}

        {/* Category 5: Development & Operations */}
        {isSectionVisible() && (
          <SectionCard
            title="Development & Operations"
            priority="infrastructure"
            icon={<WrenchScrewdriverIcon />}
            collapsible={true}
            defaultExpanded={!isCollapsed('development-operations')}
            hasChanges={false}
            loading={operationLoading?.includes('Doctor') || operationLoading?.includes('Hardware') || operationLoading?.includes('Migration')}
          >
            <div className="grid gap-8 lg:grid-cols-2">
              <InputGroup
                title="Diagnostics"
                description="System health checks and model probing"
              >
                <div className="md:col-span-2">
                  <ActionPanel>
                    <PrimaryButton
                      onClick={() => run(alphahumanDoctorReport, 'doctorReport')}
                      loading={operationLoading === 'doctorReport'}
                    >
                      Run Doctor Report
                    </PrimaryButton>
                    <PrimaryButton
                      onClick={() =>
                        run(() =>
                          alphahumanDoctorModels(providerOverride || undefined, true),
                          'probeModels'
                        )
                      }
                      loading={operationLoading === 'probeModels'}
                      variant="outline"
                    >
                      Probe Models
                    </PrimaryButton>
                  </ActionPanel>
                </div>
              </InputGroup>

              <InputGroup
                title="Hardware"
                description="Discover and introspect hardware devices"
              >
                <Field
                  label="Device Path"
                  helpText="Full path to hardware device for introspection. Common paths: /dev/tty.usbmodem* (macOS USB), /dev/ttyUSB* (Linux), COM* (Windows). Use 'Discover Devices' to find available hardware."
                  fullWidth
                >
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="Device path (e.g. /dev/tty.usbmodem)"
                    value={hardwarePath}
                    onChange={(event) => setHardwarePath(event.target.value)}
                  />
                </Field>
              </InputGroup>
            </div>

            <InputGroup
              title="Migration"
              description="Migrate data from external sources"
            >
              <Field
                label="Source Workspace"
                helpText="Path to existing agent workspace for data migration. Leave empty to migrate from default locations. Supports importing from OpenClaw, AutoGen, and other agent frameworks. Run dry-run first to preview changes."
                fullWidth
              >
                <input
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                  placeholder="Source workspace (optional)"
                  value={migrationSource}
                  onChange={(event) => setMigrationSource(event.target.value)}
                />
              </Field>
            </InputGroup>

            <ActionPanel>
              <PrimaryButton
                onClick={() => run(alphahumanHardwareDiscover, 'hardwareDiscover')}
                loading={operationLoading === 'hardwareDiscover'}
              >
                Discover Devices
              </PrimaryButton>
              <PrimaryButton
                onClick={() =>
                  run(() => alphahumanHardwareIntrospect(hardwarePath), 'hardwareIntrospect')
                }
                loading={operationLoading === 'hardwareIntrospect'}
                disabled={!hardwarePath.trim()}
                variant="outline"
              >
                Introspect Device
              </PrimaryButton>
              <PrimaryButton
                onClick={() =>
                  run(() =>
                    alphahumanMigrateOpenclaw(
                      migrationSource || undefined,
                      true
                    ),
                    'migrationDryRun'
                  )
                }
                loading={operationLoading === 'migrationDryRun'}
              >
                Dry Run Migration
              </PrimaryButton>
              <PrimaryButton
                onClick={() =>
                  run(() =>
                    alphahumanMigrateOpenclaw(
                      migrationSource || undefined,
                      false
                    ),
                    'runMigration'
                  )
                }
                loading={operationLoading === 'runMigration'}
                variant="outline"
              >
                Run Migration
              </PrimaryButton>
            </ActionPanel>
          </SectionCard>
        )}

        {/* Category 6: Interactive Tools */}
        {isSectionVisible() && (
          <SectionCard
            title="Interactive Tools"
            priority="infrastructure"
            icon={<ChatBubbleLeftRightIcon />}
            collapsible={true}
            defaultExpanded={!isCollapsed('interactive-tools')}
            hasChanges={false}
            loading={operationLoading === 'sendChat'}
          >
            {/* Agent Chat - Preserve original styling */}
            <div className="space-y-6">
              <h4 className="text-lg font-medium text-white">Agent Chat</h4>
              <div className="grid gap-4 md:grid-cols-3">
                <label className="space-y-2 text-sm text-gray-300">
                  Provider Override
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="openai"
                    value={chatProvider}
                    onChange={(event) => setChatProvider(event.target.value)}
                  />
                  <p className="text-xs text-gray-400">
                    Override the default AI provider for this chat session. Leave empty to use system default. Useful for testing different AI providers.
                  </p>
                </label>
                <label className="space-y-2 text-sm text-gray-300">
                  Model Override
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="gpt-4.1-mini"
                    value={chatModel}
                    onChange={(event) => setChatModel(event.target.value)}
                  />
                  <p className="text-xs text-gray-400">
                    Specific AI model for this chat session. Examples: gpt-4, gpt-3.5-turbo, claude-3-sonnet. Leave empty for system default.
                  </p>
                </label>
                <label className="space-y-2 text-sm text-gray-300">
                  Temperature
                  <input
                    className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200"
                    placeholder="0.7"
                    value={chatTemperature}
                    onChange={(event) => setChatTemperature(event.target.value)}
                  />
                  <p className="text-xs text-gray-400">
                    Creativity level for responses (0.0-2.0). Lower = more focused, higher = more creative. Leave empty for system default.
                  </p>
                </label>
              </div>
              <div className="space-y-3">
                <textarea
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200 min-h-[120px] resize-y"
                  placeholder="Send a message to the agent..."
                  value={chatInput}
                  onChange={(event) => setChatInput(event.target.value)}
                />
                <p className="text-xs text-gray-400 leading-relaxed">
                  Direct chat interface with the AI agent. Test conversations, debug responses, or interact with the agent using the configured settings above.
                </p>
              </div>
              <button
                className="bg-primary-600 hover:bg-primary-500 active:bg-primary-700 text-white font-medium px-6 py-3 rounded-lg transition-all duration-200 ease-in-out shadow-soft hover:shadow-medium focus:outline-none focus:ring-2 focus:ring-primary-500/50 focus:ring-offset-2 focus:ring-offset-black disabled:opacity-50 disabled:cursor-not-allowed"
                onClick={sendChat}
              >
                Send Message
              </button>
              {chatLog.length > 0 && (
                <div className="space-y-2 rounded-lg border border-white/10 bg-white/5 backdrop-blur-sm p-3">
                  {chatLog.map((entry, index) => (
                    <div
                      key={`${entry.role}-${index}`}
                      className={`text-sm ${
                        entry.role === 'user' ? 'text-white' : 'text-emerald-200'
                      }`}
                    >
                      <span className="font-semibold uppercase text-[10px] tracking-wide">
                        {entry.role}
                      </span>
                      <div className="whitespace-pre-wrap">{entry.text}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Output Console */}
            <div className="space-y-6">
              <div>
                <h4 className="text-lg font-medium text-white flex items-center gap-2">
                  <DocumentTextIcon className="h-5 w-5" />
                  Output Console
                </h4>
                <p className="text-sm text-gray-400 mt-2">
                  Real-time command results and system responses. Shows JSON output, error messages, and operation status from all Tauri commands.
                </p>
              </div>
              {error && (
                <div className="rounded-lg border border-coral-500/40 bg-coral-500/10 px-4 py-3 text-sm text-coral-200">
                  {error}
                </div>
              )}
              <div className="space-y-3">
                <textarea
                  className="w-full px-4 py-3 rounded-lg bg-stone-900/40 border border-stone-800/60 text-white placeholder-stone-400 focus:border-primary-500/50 focus:ring-2 focus:ring-primary-500/30 focus:outline-none transition-all duration-200 min-h-[240px] font-mono text-xs resize-y"
                  value={output}
                  readOnly
                  placeholder="Command output will appear here..."
                />
                <p className="text-xs text-gray-400 leading-relaxed">
                  Read-only console showing formatted JSON responses, error details, and debugging information from system operations.
                </p>
              </div>
            </div>
          </SectionCard>
        )}
      </div>
    </div>
  );
};

export default TauriCommandsPanel;
