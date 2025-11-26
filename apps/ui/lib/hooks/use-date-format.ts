import { useMemo } from 'react'
import { usePreferences } from '@/lib/queries'
import {
  formatDateTimeFull,
  formatDateOnly,
  formatRelativeTime,
} from '@/lib/utils/format'

/**
 * Get browser's local timezone
 */
function getBrowserTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone
  } catch {
    return 'UTC'
  }
}

/**
 * Hook to format dates and times according to user preferences
 * Provides formatting functions that automatically use the user's timezone and date format preferences
 */
export function useDateFormat() {
  const { data: preferencesData } = usePreferences()
  const preferences = preferencesData?.preferences

  // Use browser's local timezone as default instead of UTC
  const timezone = preferences?.timezone || getBrowserTimezone()
  const dateFormat = preferences?.date_format || 'iso'

  const formatters = useMemo(() => {
    return {
      /**
       * Format a date and time (full format)
       */
      formatDateTime: (date: Date | string) => formatDateTimeFull(date, timezone, dateFormat),
      
      /**
       * Format a date only (no time)
       */
      formatDate: (date: Date | string) => formatDateOnly(date, dateFormat),
      
      /**
       * Format a time only
       */
      formatTime: (date: Date | string) => {
        const d = typeof date === 'string' ? new Date(date) : date
        if (isNaN(d.getTime())) return 'Invalid Date'
        try {
          return d.toLocaleTimeString('en-US', {
            timeZone: timezone,
            hour12: false,
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
          })
        } catch {
          return d.toLocaleTimeString('en-US', {
            hour12: false,
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
          })
        }
      },
      
      /**
       * Format relative time (e.g., "2h ago", "3d ago")
       * Falls back to formatted date for older dates
       */
      formatRelative: (date: Date | string) => formatRelativeTime(
        typeof date === 'string' ? date : date.toISOString(),
        timezone,
        dateFormat
      ),
      
      /**
       * Get current timezone
       */
      getTimezone: () => timezone,
      
      /**
       * Get current date format
       */
      getDateFormat: () => dateFormat,
    }
  }, [timezone, dateFormat])

  return formatters
}

