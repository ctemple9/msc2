// Parses raw console lines into the in-game chat feed MSC 1's
// OverviewChatCardView.ChatFeedParser derives (chat / advancement / join /
// leave). Java logs chat to the console; Bedrock (BDS) does not, so only
// connect/disconnect lines appear there.
import type { Schema } from '../shared/types';

export type ChatFeedMessage = {
  id: string;
  kind: 'chat' | 'advancement' | 'join' | 'leave';
  player: string | null;
  text: string;
  ts: string;
};

const ADVANCEMENT_MARKERS = [
  ' has made the advancement ',
  ' has completed the challenge ',
  ' has reached the goal ',
];

function strippedPayload(raw: string): string {
  const marker = ']: ';
  const idx = raw.lastIndexOf(marker);
  return idx === -1 ? raw : raw.slice(idx + marker.length);
}

function parseLine(raw: string, ts: string, index: number): ChatFeedMessage | null {
  let payload = strippedPayload(raw);
  if (payload.startsWith('[Not Secure] ')) payload = payload.slice('[Not Secure] '.length);
  payload = payload.trim();
  if (!payload) return null;

  if (payload.startsWith('<')) {
    const gt = payload.indexOf('>');
    if (gt !== -1) {
      const name = payload.slice(1, gt).trim();
      const text = payload.slice(gt + 1).trim();
      if (name && text) return { id: `${index}`, kind: 'chat', player: name, text, ts };
    }
  }

  for (const marker of ADVANCEMENT_MARKERS) {
    const at = payload.indexOf(marker);
    if (at !== -1) {
      const name = payload.slice(0, at).trim();
      let text = payload.slice(at + marker.length).trim();
      if (text.startsWith('[') && text.endsWith(']')) text = text.slice(1, -1);
      if (name && name.length < 40) {
        return { id: `${index}`, kind: 'advancement', player: name, text, ts };
      }
    }
  }

  if (payload.endsWith(' joined the game')) {
    return {
      id: `${index}`,
      kind: 'join',
      player: payload.slice(0, -' joined the game'.length),
      text: 'joined the game',
      ts,
    };
  }
  if (payload.endsWith(' left the game')) {
    return {
      id: `${index}`,
      kind: 'leave',
      player: payload.slice(0, -' left the game'.length),
      text: 'left the game',
      ts,
    };
  }

  const connected = payload.indexOf('Player connected: ');
  if (connected !== -1) {
    const rest = payload.slice(connected + 'Player connected: '.length);
    const name = (rest.split(',')[0] ?? rest).trim();
    return { id: `${index}`, kind: 'join', player: name, text: 'connected', ts };
  }
  const disconnected = payload.indexOf('Player disconnected: ');
  if (disconnected !== -1) {
    const rest = payload.slice(disconnected + 'Player disconnected: '.length);
    const name = (rest.split(',')[0] ?? rest).trim();
    return { id: `${index}`, kind: 'leave', player: name, text: 'disconnected', ts };
  }

  return null;
}

export function parseChatFeed(lines: readonly Schema['ConsoleLineDTO'][]): ChatFeedMessage[] {
  const out: ChatFeedMessage[] = [];
  lines.forEach((line, index) => {
    const message = parseLine(line.text, line.ts, index);
    if (message) out.push(message);
  });
  return out;
}
