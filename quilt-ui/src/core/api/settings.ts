import type {
  DateFormatOption,
  DismissTourRequest,
  TourStateResponse,
  UpdateSettingsRequest,
  UserSettings,
} from '@shared/types/api'
import { cachedFetch, fetchJson, invalidateAllCache, invalidateCacheKey } from './client'

export const settingsApi = {
  getSettings: () => cachedFetch<UserSettings>('GET', '/settings'),

  updateSettings: (data: UpdateSettingsRequest) =>
    fetchJson<UserSettings>('/settings', { method: 'PUT', body: JSON.stringify(data) }).then(settings => {
      invalidateAllCache()
      return settings
    }),

  getDateFormats: () => cachedFetch<DateFormatOption[]>('GET', '/settings/formats'),

  getTourState: () => cachedFetch<TourStateResponse>('GET', '/user/tour-state'),

  dismissTour: (tourName: string) =>
    fetchJson<TourStateResponse>('/user/tour-state/dismiss', {
      method: 'POST',
      body: JSON.stringify({ tour: tourName } satisfies DismissTourRequest),
    }).then(result => {
      invalidateCacheKey('GET /user/tour-state')
      return result
    }),
}
