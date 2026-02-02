import { useEffect, useState } from 'react';

import {
  detectPlatform,
  fetchLatestRelease,
  getDownloadLink,
  getPlatformDisplayName,
  parseReleaseAssets,
  parseReleaseAssetsByArchitecture,
  type Architecture,
  type ArchitectureDownloadLink,
  type Platform,
  type PlatformArchitectureLinks,
  type PlatformDownloadLinks,
  type PlatformInfo,
} from '../utils/deviceDetection';
import { isTauri } from '../utils/tauriCommands';

interface DownloadOption {
  platform: Platform;
  label: string;
  icon: string;
}

const downloadOptions: DownloadOption[] = [
  { platform: 'windows', label: 'Windows', icon: '🪟' },
  { platform: 'macos', label: 'macOS', icon: '🍎' },
  { platform: 'linux', label: 'Linux', icon: '🐧' },
];

const DownloadScreen = () => {
  const [platformInfo, setPlatformInfo] = useState<PlatformInfo | null>(null);
  const [selectedPlatform, setSelectedPlatform] = useState<Platform | null>(null);
  const [selectedArchitecture, setSelectedArchitecture] = useState<Architecture | null>(null);
  const [releaseLinks, setReleaseLinks] = useState<PlatformDownloadLinks | null>(null);
  const [architectureLinks, setArchitectureLinks] = useState<PlatformArchitectureLinks | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Only show download screen on web (not in Tauri app)
    if (isTauri()) {
      return;
    }

    const detected = detectPlatform();
    setPlatformInfo(detected);
    setSelectedPlatform(detected.platform);
    setSelectedArchitecture(detected.architecture);

    // Fetch latest release from GitHub
    const loadReleaseLinks = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const release = await fetchLatestRelease();
        const links = parseReleaseAssets(release.assets);
        const archLinks = parseReleaseAssetsByArchitecture(release.assets);
        setReleaseLinks(links);
        setArchitectureLinks(archLinks);

        // Auto-select best architecture for detected platform
        const platformArchLinks = archLinks[detected.platform as keyof PlatformArchitectureLinks];
        if (platformArchLinks && platformArchLinks.length > 0) {
          // Prefer detected architecture, otherwise use first available
          const preferredLink = platformArchLinks.find(link => link.architecture === detected.architecture) || platformArchLinks[0];
          setSelectedArchitecture(preferredLink.architecture);
        }
      } catch (err) {
        console.error('Failed to fetch release links:', err);
        setError(err instanceof Error ? err.message : 'Failed to load download links');
      } finally {
        setIsLoading(false);
      }
    };

    loadReleaseLinks();
  }, []);

  // Don't render if running in Tauri or platform not detected
  if (isTauri() || !platformInfo || !selectedPlatform) {
    return null;
  }

  // Get download URL for selected platform and architecture
  const getDownloadUrl = (): string => {
    if (!selectedPlatform || !architectureLinks) {
      return getDownloadLink(selectedPlatform || 'unknown', releaseLinks || undefined);
    }

    const platformArchLinks = architectureLinks[selectedPlatform as keyof PlatformArchitectureLinks];
    if (platformArchLinks && selectedArchitecture) {
      const link = platformArchLinks.find(l => l.architecture === selectedArchitecture);
      if (link) {
        return link.url;
      }
      // Fallback to first available architecture
      if (platformArchLinks.length > 0) {
        return platformArchLinks[0].url;
      }
    }

    return getDownloadLink(selectedPlatform, releaseLinks || undefined);
  };

  const downloadUrl = getDownloadUrl();
  const platformName = getPlatformDisplayName(selectedPlatform || 'unknown');

  const handleDownload = () => {
    window.open(downloadUrl, '_blank');
  };

  // Get available architectures for selected platform
  const getAvailableArchitectures = (): ArchitectureDownloadLink[] => {
    if (!selectedPlatform || !architectureLinks) {
      return [];
    }
    return architectureLinks[selectedPlatform as keyof PlatformArchitectureLinks] || [];
  };

  const availableArchitectures = getAvailableArchitectures();
  const hasMultipleArchitectures = availableArchitectures.length > 1;

  return (
    <div className="animate-fade-up">
      {/* Loading state */}
      {isLoading && (
        <div className="mb-6">
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-xl p-4">
            <div className="flex items-center justify-center space-x-3">
              <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-500"></div>
              <p className="text-sm opacity-70">Loading download links...</p>
            </div>
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !isLoading && (
        <div className="mb-6">
          <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-4">
            <p className="text-sm text-red-400">
              {error}. Using fallback download links.
            </p>
          </div>
        </div>
      )}

      {/* Auto-detected platform */}
      {!isLoading && (
        <div className="mb-6">
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-xl p-4 mb-4">
            <div className="flex flex-col space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <span className="text-2xl">
                    {downloadOptions.find(opt => opt.platform === selectedPlatform)?.icon}
                  </span>
                  <div>
                    <p className="text-sm opacity-70">Recommended for you</p>
                    <p className="font-semibold">{platformName}</p>
                  </div>
                </div>
                <button
                  onClick={handleDownload}
                  disabled={!downloadUrl || downloadUrl.includes('example.com')}
                  className="bg-blue-500 hover:bg-blue-600 active:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-white font-semibold px-6 py-2 rounded-lg transition-all duration-300 hover:shadow-medium hover:scale-[1.02] active:scale-[0.98]">
                  Download
                </button>
              </div>

              {/* Architecture selection */}
              {hasMultipleArchitectures && (
                <div className="border-t border-blue-500/20 pt-4">
                  <p className="text-xs opacity-70 mb-2">Select architecture:</p>
                  <div className="flex flex-wrap gap-2">
                    {availableArchitectures.map(archLink => {
                      const isSelected = selectedArchitecture === archLink.architecture;
                      const isRecommended = platformInfo?.architecture === archLink.architecture;
                      return (
                        <button
                          key={archLink.architecture}
                          onClick={() => setSelectedArchitecture(archLink.architecture)}
                          className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-all duration-200 ${
                            isSelected
                              ? 'bg-blue-500 text-white'
                              : 'bg-white/5 hover:bg-white/10 text-white/70 hover:text-white border border-white/10 hover:border-white/20'
                          }`}>
                          {archLink.displayName}
                          {isRecommended && !isSelected && (
                            <span className="ml-1.5 text-[10px] opacity-60">(recommended)</span>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Other platforms */}
      {!isLoading && (
        <div className="border-t border-white/10 pt-6">
          <p className="text-sm opacity-70 mb-4 text-center">Or download for other platforms:</p>
          <div className="grid grid-cols-2 gap-3">
            {downloadOptions
              .filter(opt => opt.platform !== selectedPlatform)
              .map(option => {
                const platformArchLinks = architectureLinks?.[option.platform as keyof PlatformArchitectureLinks];
                const hasValidLink = platformArchLinks && platformArchLinks.length > 0;
                const defaultLink = platformArchLinks?.[0]?.url || getDownloadLink(option.platform, releaseLinks || undefined);
                const hasMultipleArchs = platformArchLinks && platformArchLinks.length > 1;

                return (
                  <div key={option.platform} className="flex flex-col space-y-2">
                    <button
                      onClick={() => {
                        if (hasValidLink) {
                          setSelectedPlatform(option.platform);
                          // Auto-select first architecture for the platform
                          if (platformArchLinks && platformArchLinks.length > 0) {
                            setSelectedArchitecture(platformArchLinks[0].architecture);
                          }
                          window.open(defaultLink, '_blank');
                        }
                      }}
                      disabled={!hasValidLink}
                      className="flex items-center justify-center space-x-2 bg-white/5 hover:bg-white/10 disabled:opacity-50 disabled:cursor-not-allowed border border-white/10 hover:border-white/20 rounded-lg p-3 transition-all duration-300 hover:scale-[1.02] active:scale-[0.98]">
                      <span className="text-xl">{option.icon}</span>
                      <span className="text-sm font-medium">{option.label}</span>
                    </button>
                    {hasMultipleArchs && (
                      <div className="flex flex-wrap gap-1.5 justify-center">
                        {platformArchLinks.map(archLink => (
                          <button
                            key={archLink.architecture}
                            onClick={() => {
                              setSelectedPlatform(option.platform);
                              setSelectedArchitecture(archLink.architecture);
                              window.open(archLink.url, '_blank');
                            }}
                            className="px-2 py-0.5 text-[10px] font-medium rounded bg-white/5 hover:bg-white/10 text-white/60 hover:text-white/80 border border-white/10 hover:border-white/20 transition-all">
                            {archLink.displayName}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
          </div>
        </div>
      )}
    </div>
  );
};

export default DownloadScreen;
