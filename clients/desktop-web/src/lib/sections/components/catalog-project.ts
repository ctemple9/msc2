import type { Schema, ScreenApi } from '../shared/types';
import { catalogDetailPaths, type ProjectDetailItem } from './model';

export type CatalogProvider = 'modrinth' | 'curseforge';
export interface CatalogProjectPage {
  project: Schema['CatalogProjectDetailDTO'];
  versions: Schema['CatalogVersionDTO'][];
  releases: Schema['ProviderCatalogReleaseDTO'][];
  nextOffset?: number;
  followersKnown: boolean;
  serverSideKnown: boolean;
  distributionAllowed?: boolean | null;
}

/** Provider differences end here; the project sheet owns one layout and one
 * version-row renderer for both catalogs. Missing metadata stays unknown. */
export async function loadCatalogProjectPage(
  api: ScreenApi,
  provider: CatalogProvider,
  item: ProjectDetailItem,
  offset = 0,
): Promise<CatalogProjectPage> {
  if (provider === 'modrinth') {
    const [project, response] = await Promise.all([
      api.get<Schema['CatalogProjectDetailDTO']>(catalogDetailPaths.project(item.projectId)),
      api.get<Schema['CatalogVersionsResponseDTO']>(catalogDetailPaths.versions(item.projectId)),
    ]);
    return {
      project,
      versions: response.versions ?? [],
      releases: [],
      followersKnown: true,
      serverSideKnown: true,
    };
  }
  const base = `/v1/catalog/curseforge/projects/${encodeURIComponent(item.projectId)}`;
  const [detail, response] = await Promise.all([
    api.get<Schema['ProviderCatalogProjectDTO']>(base),
    api.get<Schema['ProviderCatalogVersionsDTO']>(
      `${base}/versions?offset=${offset}&kind=${encodeURIComponent(item.projectType ?? 'mod')}`,
    ),
  ]);
  const next = offset + response.versions.length;
  return {
    project: detail.project,
    versions: response.versions.map((release) => release.version),
    releases: response.versions,
    nextOffset: response.versions.length && next < response.totalCount ? next : undefined,
    followersKnown: false,
    serverSideKnown: false,
    distributionAllowed: detail.distributionAllowed,
  };
}
