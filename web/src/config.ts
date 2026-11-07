export interface SiteConfig {
  base: string;
  version: string;
}

export const repository = 'https://github.com/Zxilly/cjr';

export const downloads = [
  { label: 'windows', asset: 'x86_64-pc-windows-msvc.zip' },
  { label: 'linux', asset: 'x86_64-unknown-linux-musl.tar.gz' },
  { label: 'macArm', asset: 'aarch64-apple-darwin.tar.gz' },
  { label: 'macIntel', asset: 'x86_64-apple-darwin.tar.gz' },
] as const;

export function releaseUrl(version: string) {
  return `${repository}/releases/tag/v${version}`;
}

export function downloadUrl(version: string, asset: string) {
  return `${repository}/releases/download/v${version}/cjr-v${version}-${asset}`;
}

export function installCommand(version: string) {
  return `cargo install --git ${repository} --tag v${version} --locked`;
}
