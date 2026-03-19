'use client';

import { useEffect, useState } from 'react';
import {
  type Architecture,
  type ArchitectureDownloadLink,
  detectPlatform,
  fetchLatestRelease,
  getDownloadLink,
  getPlatformDisplayName,
  parseReleaseAssets,
  parseReleaseAssetsByArchitecture,
  type Platform,
  type PlatformArchitectureLinks,
  type PlatformDownloadLinks,
  type PlatformInfo,
} from '@/lib/deviceDetection';

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

interface DownloadModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function DownloadModal({ isOpen, onClose }: DownloadModalProps) {
  const [platformInfo, setPlatformInfo] = useState<PlatformInfo | null>(null);
  const [selectedPlatform, setSelectedPlatform] = useState<Platform | null>(null);
  const [selectedArchitecture, setSelectedArchitecture] = useState<Architecture | null>(null);
  const [releaseLinks, setReleaseLinks] = useState<PlatformDownloadLinks | null>(null);
  const [architectureLinks, setArchitectureLinks] = useState<PlatformArchitectureLinks | null>(
    null
  );
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;

    const detected = detectPlatform();
    setPlatformInfo(detected);
    setSelectedPlatform(detected.platform);
    setSelectedArchitecture(detected.architecture);

    const loadReleaseLinks = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const release = await fetchLatestRelease();
        const links = parseReleaseAssets(release.assets);
        const archLinks = parseReleaseAssetsByArchitecture(release.assets);
        setReleaseLinks(links);
        setArchitectureLinks(archLinks);

        const platformArchLinks = archLinks[detected.platform as keyof PlatformArchitectureLinks];
        if (platformArchLinks && platformArchLinks.length > 0) {
          const preferredLink =
            platformArchLinks.find(link => link.architecture === detected.architecture) ||
            platformArchLinks[0];
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
  }, [isOpen]);

  const getDownloadUrl = (): string => {
    if (!selectedPlatform || !architectureLinks) {
      return getDownloadLink(selectedPlatform || 'unknown', releaseLinks || undefined);
    }

    const platformArchLinks =
      architectureLinks[selectedPlatform as keyof PlatformArchitectureLinks];
    if (platformArchLinks && selectedArchitecture) {
      const link = platformArchLinks.find(l => l.architecture === selectedArchitecture);
      if (link) {
        return link.url;
      }
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

  const getAvailableArchitectures = (): ArchitectureDownloadLink[] => {
    if (!selectedPlatform || !architectureLinks) {
      return [];
    }
    return architectureLinks[selectedPlatform as keyof PlatformArchitectureLinks] || [];
  };

  const availableArchitectures = getAvailableArchitectures();
  const hasMultipleArchitectures = availableArchitectures.length > 1;

  const showRecommended =
    platformInfo &&
    selectedPlatform &&
    (selectedPlatform === 'windows' ||
      selectedPlatform === 'macos' ||
      selectedPlatform === 'linux' ||
      selectedPlatform === 'android' ||
      selectedPlatform === 'ios' ||
      selectedPlatform === 'unknown');

  useEffect(() => {
    if (!isOpen) return;
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="download-modal-title"
    >
      <div
        className="absolute inset-0 bg-black/70 backdrop-blur-sm"
        onClick={onClose}
        onKeyDown={e => e.key === 'Escape' && onClose()}
      />
      <div className="relative z-10 w-full max-w-lg rounded-xl border border-zinc-800 bg-zinc-900 shadow-xl transition-all duration-200">
        <div className="flex items-center justify-between border-b border-zinc-800 px-6 py-4">
          <h2 id="download-modal-title" className="text-lg font-semibold text-white">
            Download OpenHuman
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg p-1.5 text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-white"
            aria-label="Close"
          >
            <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <div className="max-h-[70vh] overflow-y-auto px-6 py-5">
          {isLoading && (
            <div className="mb-6">
              <div className="flex items-center justify-center space-x-3 rounded-xl border border-zinc-700 bg-zinc-800/50 p-4">
                <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-500 border-t-white" />
                <p className="text-sm text-zinc-400">Loading download links...</p>
              </div>
            </div>
          )}

          {error && !isLoading && (
            <div className="mb-6">
              <div className="rounded-xl border border-red-500/20 bg-red-500/10 p-4">
                <p className="text-sm text-red-400">
                  {error}. Using fallback download links.
                </p>
              </div>
            </div>
          )}

          {!isLoading && showRecommended && (
            <div className="mb-6">
              <div className="mb-4 rounded-xl border border-zinc-700 bg-zinc-800/50 p-4">
                <div className="flex flex-col space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-3">
                      <span className="text-2xl">
                        {downloadOptions.find(opt => opt.platform === selectedPlatform)?.icon ?? '💻'}
                      </span>
                      <div>
                        <p className="text-sm text-zinc-400">Recommended for you</p>
                        <p className="font-semibold text-white">{platformName}</p>
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={handleDownload}
                      disabled={!downloadUrl || downloadUrl.includes('example.com')}
                      className="rounded-lg bg-white px-6 py-2 font-semibold text-zinc-950 transition-all duration-300 hover:scale-[1.02] hover:bg-zinc-200 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      Download
                    </button>
                  </div>

                  {hasMultipleArchitectures && (
                    <div className="border-t border-zinc-700 pt-4">
                      <p className="mb-2 text-xs text-zinc-400">Select architecture:</p>
                      <div className="flex flex-wrap gap-2">
                        {availableArchitectures.map(archLink => {
                          const isSelected = selectedArchitecture === archLink.architecture;
                          const isRecommended = platformInfo?.architecture === archLink.architecture;
                          return (
                            <button
                              key={archLink.architecture}
                              type="button"
                              onClick={() => setSelectedArchitecture(archLink.architecture)}
                              className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-all duration-200 ${
                                isSelected
                                  ? 'bg-white text-zinc-900'
                                  : 'border border-zinc-600 bg-zinc-800/50 text-zinc-300 hover:border-zinc-500 hover:bg-zinc-700/50'
                              }`}
                            >
                              {archLink.displayName}
                              {isRecommended && !isSelected && (
                                <span className="ml-1.5 text-[10px] text-zinc-500">
                                  (recommended)
                                </span>
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

          {!isLoading && (
            <div className="border-t border-zinc-800 pt-6">
              <p className="mb-4 text-center text-sm text-zinc-400">
                Or download for other platforms:
              </p>
              <div className="grid grid-cols-2 gap-3">
                {downloadOptions
                  .filter(opt => opt.platform !== selectedPlatform)
                  .map(option => {
                    const platformArchLinks =
                      architectureLinks?.[option.platform as keyof PlatformArchitectureLinks];
                    const hasValidLink = platformArchLinks && platformArchLinks.length > 0;
                    const defaultLink =
                      platformArchLinks?.[0]?.url ||
                      getDownloadLink(option.platform, releaseLinks || undefined);
                    const hasMultipleArchs = platformArchLinks && platformArchLinks.length > 1;

                    return (
                      <div key={option.platform} className="flex flex-col space-y-2">
                        <button
                          type="button"
                          onClick={() => {
                            if (hasValidLink) {
                              setSelectedPlatform(option.platform);
                              if (platformArchLinks && platformArchLinks.length > 0) {
                                setSelectedArchitecture(platformArchLinks[0].architecture);
                              }
                              window.open(defaultLink, '_blank');
                            }
                          }}
                          disabled={!hasValidLink}
                          className="flex items-center justify-center space-x-2 rounded-lg border border-zinc-700 bg-zinc-800/50 p-3 font-medium text-white transition-all duration-300 hover:scale-[1.02] hover:border-zinc-600 hover:bg-zinc-700/50 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          <span className="text-xl">{option.icon}</span>
                          <span className="text-sm">{option.label}</span>
                        </button>
                        {hasMultipleArchs && platformArchLinks && (
                          <div className="flex flex-wrap justify-center gap-1.5">
                            {platformArchLinks.map(archLink => (
                              <button
                                key={archLink.architecture}
                                type="button"
                                onClick={() => {
                                  setSelectedPlatform(option.platform);
                                  setSelectedArchitecture(archLink.architecture);
                                  window.open(archLink.url, '_blank');
                                }}
                                className="rounded border border-zinc-600 bg-zinc-800/50 px-2 py-0.5 text-[10px] font-medium text-zinc-400 transition-colors hover:border-zinc-500 hover:text-zinc-300"
                              >
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
      </div>
    </div>
  );
}
