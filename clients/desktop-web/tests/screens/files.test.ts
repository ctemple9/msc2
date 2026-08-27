import { describe, expect, it } from 'vitest';
import {
  breadcrumbsFor,
  browseNoticeFor,
  filePaths,
  readErrorMessage,
  relativeTime,
} from '../../src/lib/sections/files/model';

describe('routes -- ServerFilesTabView.swift is the real oracle', () => {
  it('exposes the real, frozen files routes (P12.9 backs them with crates/msc-agent/src/routes/files.rs)', () => {
    expect(filePaths.browse('')).toBe('/v1/files');
    expect(filePaths.browse('plugins')).toBe('/v1/files?path=plugins');
    expect(filePaths.browse('a b/c')).toBe('/v1/files?path=a%20b%2Fc');
    expect(filePaths.read('server.properties')).toBe('/v1/files/read?path=server.properties');
  });
});

describe('breadcrumbs -- ServerFilesTabView.swift breadcrumbs', () => {
  it('is just "Server Root" at the root', () => {
    expect(breadcrumbsFor('')).toEqual([{ label: 'Server Root', path: '' }]);
  });

  it('adds one crumb per path segment, building each prefix', () => {
    expect(breadcrumbsFor('plugins/config')).toEqual([
      { label: 'Server Root', path: '' },
      { label: 'plugins', path: 'plugins' },
      { label: 'config', path: 'plugins/config' },
    ]);
  });
});

describe('relative time', () => {
  it('is undefined for a missing or unparsable timestamp', () => {
    expect(relativeTime(undefined)).toBeUndefined();
    expect(relativeTime('not-a-date')).toBeUndefined();
  });

  it('buckets a recent timestamp into a short relative phrase', () => {
    const twoMinutesAgo = new Date(Date.now() - 2 * 60 * 1000).toISOString();
    expect(relativeTime(twoMinutesAgo)).toBe('2m ago');
  });
});

describe('browse/read outcome messages', () => {
  it('turns a 200-with-note browse response into an explanation', () => {
    expect(browseNoticeFor('invalid_path')).toBe('That path is outside the server directory.');
    expect(browseNoticeFor('directory_not_found')).toBe("That folder doesn't exist.");
    expect(browseNoticeFor(undefined)).toBeUndefined();
  });

  it("turns a failed read's error message into an explanation", () => {
    expect(readErrorMessage('not_previewable')).toBe("This file type can't be previewed.");
    expect(readErrorMessage('directory_not_file')).toBe("That's a folder, not a file.");
    expect(readErrorMessage('file_not_found')).toBe('That file no longer exists.');
    expect(readErrorMessage('read_failed')).toBe('The file could not be read.');
    expect(readErrorMessage(undefined)).toBe('Could not open this file.');
  });
});
