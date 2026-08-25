// Personal app identity for the sidebar avatar — MSC 1's minecraftUsername /
// minecraftBedrockGamertag / minecraftAvatarEditionRawValue config fields. This
// is the human's own identity, not agent-owned server data, so it is global
// (not host/server-scoped) and held client-locally until a real field exists —
// the same treatment P12.1 gave bannerColor (see ../styles/bannerColor.ts).
export type AvatarEdition = 'java' | 'bedrock';

const EDITION_KEY = 'msc2.player.edition';
const JAVA_KEY = 'msc2.player.javaUsername';
const BEDROCK_KEY = 'msc2.player.bedrockGamertag';

function readKey(key: string): string {
  if (typeof localStorage === 'undefined') return '';
  return localStorage.getItem(key) ?? '';
}

function writeKey(key: string, value: string): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(key, value);
}

export function getStoredEdition(): AvatarEdition {
  return readKey(EDITION_KEY) === 'bedrock' ? 'bedrock' : 'java';
}

export function setStoredEdition(edition: AvatarEdition): void {
  writeKey(EDITION_KEY, edition);
}

export function getIdentity(edition: AvatarEdition): string {
  return readKey(edition === 'java' ? JAVA_KEY : BEDROCK_KEY);
}

export function setIdentity(edition: AvatarEdition, value: string): void {
  writeKey(edition === 'java' ? JAVA_KEY : BEDROCK_KEY, value);
}
