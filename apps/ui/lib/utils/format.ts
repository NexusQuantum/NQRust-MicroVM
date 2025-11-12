export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return "0 Bytes"

  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ["Bytes", "KB", "MB", "GB", "TB"]

  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return Number.parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i]
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return `${hours}h ${minutes}m`
}

/**
 * Format a date according to the specified date format
 * @param date - Date object or date string
 * @param format - Format type: 'iso', 'us', or 'eu'
 * @returns Formatted date string
 */
function formatDateInternal(date: Date | string, format: string = 'iso'): string {
  const d = typeof date === 'string' ? new Date(date) : date
  
  if (isNaN(d.getTime())) {
    return 'Invalid Date'
  }

  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')

  switch (format) {
    case 'us':
      return `${month}/${day}/${year}`
    case 'eu':
      return `${day}/${month}/${year}`
    case 'iso':
    default:
      return `${year}-${month}-${day}`
  }
}

/**
 * Format a date and time according to timezone and date format preferences
 * @param date - Date object or date string
 * @param timezone - IANA timezone string (e.g., 'America/New_York', 'UTC')
 * @param dateFormat - Date format type: 'iso', 'us', or 'eu'
 * @param includeTime - Whether to include time in the output
 * @returns Formatted date/time string
 */
export function formatDateTime(
  date: Date | string,
  timezone: string = 'UTC',
  dateFormat: string = 'iso',
  includeTime: boolean = true
): string {
  const d = typeof date === 'string' ? new Date(date) : date
  
  if (isNaN(d.getTime())) {
    return 'Invalid Date'
  }

  try {
    // Format date part
    const dateStr = formatDateInternal(d, dateFormat)
    
    if (!includeTime) {
      return dateStr
    }

    // Format time part with timezone
    const timeStr = d.toLocaleTimeString('en-US', {
      timeZone: timezone,
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })

    return `${dateStr} ${timeStr}`
  } catch (error) {
    // Fallback to default formatting if timezone is invalid
    const dateStr = formatDateInternal(d, dateFormat)
    if (!includeTime) {
      return dateStr
    }
    const timeStr = d.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
    return `${dateStr} ${timeStr}`
  }
}

/**
 * Format a date and time for display (full format with timezone)
 * @param date - Date object or date string
 * @param timezone - IANA timezone string
 * @param dateFormat - Date format type: 'iso', 'us', or 'eu'
 * @returns Formatted date/time string
 */
export function formatDateTimeFull(
  date: Date | string,
  timezone: string = 'UTC',
  dateFormat: string = 'iso'
): string {
  return formatDateTime(date, timezone, dateFormat, true)
}

/**
 * Format a date only (no time)
 * @param date - Date object or date string
 * @param dateFormat - Date format type: 'iso', 'us', or 'eu'
 * @returns Formatted date string
 */
export function formatDateOnly(
  date: Date | string,
  dateFormat: string = 'iso'
): string {
  return formatDateInternal(date, dateFormat)
}

/**
 * Format relative time (e.g., "2h ago", "3d ago")
 * Uses user preferences for timezone when displaying full dates
 * @param dateString - ISO date string
 * @param timezone - IANA timezone string (optional, for fallback to full date)
 * @param dateFormat - Date format type (optional, for fallback to full date)
 * @returns Relative time string or formatted date
 */
export function formatRelativeTime(
  dateString: string,
  timezone?: string,
  dateFormat?: string
): string {
  const date = new Date(dateString)
  const now = new Date()
  
  if (isNaN(date.getTime())) {
    return 'Invalid Date'
  }

  const diffMs = now.getTime() - date.getTime()
  const diffSecs = Math.floor(diffMs / 1000)
  const diffMins = Math.floor(diffSecs / 60)
  const diffHours = Math.floor(diffMins / 60)
  const diffDays = Math.floor(diffHours / 24)

  if (diffSecs < 60) return "just now"
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 7) return `${diffDays}d ago`

  // For dates older than a week, use formatted date with preferences
  if (timezone && dateFormat) {
    return formatDateOnly(date, dateFormat)
  }
  return date.toLocaleDateString()
}

export function formatPercentage(value: number, decimals = 1): string {
  return `${value.toFixed(decimals)}%`
}

export function generateRequestId(): string {
  return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15)
}
