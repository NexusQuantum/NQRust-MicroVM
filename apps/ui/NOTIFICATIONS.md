# Function State Notifications

Sistem notifikasi untuk memantau **SEMUA** perubahan state function dan memberikan feedback real-time kepada user.

## Komponen

### 1. **useFunctionStateMonitor Hook**
Location: `/lib/hooks/use-function-state-monitor.ts`

Hook ini memantau perubahan state dari sebuah function dan mengirim notifikasi ketika state berubah.

**Features:**
- Polling function state setiap 3 detik
- Auto-stop polling ketika function mencapai terminal state (stable states)
- Mengirim notifikasi untuk **SETIAP** perubahan state
- **Special handling untuk error states**: Mengirim notifikasi bahkan jika function langsung error tanpa melalui state transisi
- Support callback untuk custom handling
- Fallback untuk state yang belum terdefinisi

**Terminal States** (polling berhenti):
- `ready` - Function siap digunakan
- `stopped` - Function dihentikan
- `inactive` - Function tidak aktif
- `failed` - Deployment gagal
- `error` - Function error
- `crashed` - Function crash

**Usage:**
```typescript
const { currentState, functionData } = useFunctionStateMonitor({
  functionId: 'function-123',
  enabled: true,
  onStateChange: (oldState, newState) => {
    console.log(`State changed from ${oldState} to ${newState}`)
  }
})
```

### 2. **FunctionStateMonitorProvider**
Location: `/components/providers/function-state-monitor-provider.tsx`

Provider yang memantau semua functions yang membutuhkan state tracking.

**Monitoring Strategy:**
Provider akan memonitor functions yang memenuhi salah satu kriteria:
1. **Functions dalam transitional states** (NOT in stable states: ready, stopped, inactive)
   - Termasuk: creating, booting, deploying, starting, running, error, failed, crashed, dll.
2. **Functions yang baru dibuat** (< 10 menit) terlepas dari state-nya
   - Memastikan kita tidak melewatkan notifikasi untuk functions yang error sangat cepat

**Why Monitor Recent Functions:**
- Function bisa error sangat cepat (< 1 detik) setelah dibuat
- Dengan memantau semua functions yang baru dibuat dalam 10 menit, kita ensure:
  - Tidak ada transisi state yang terlewat
  - Error notifications selalu terkirim
  - Retry attempts di-track

**Integration:**
Provider ini sudah diintegrasikan di dashboard layout (`app/(dashboard)/layout.tsx`).

### 3. **Notification Store Integration**
Location: `/lib/queries.ts` - `useCreateFunction`

Ketika function baru dibuat, notifikasi awal akan ditambahkan ke notification store.

## State Transition Flow

Semua perubahan state akan direkam sebagai notifikasi:

```
Creating → Booting → Deploying → Starting → Ready
   ↓          ↓          ↓          ↓         ↓
 [Info]    [Info]     [Info]    [Info]   [Success]

atau

Creating → Error (langsung!)
   ↓          ↓
 [Info]   [Error] ✅ Notification tetap terkirim!

atau

Running → Stopping → Stopped
   ↓          ↓          ↓
[Success] [Warning] [Warning]
```

### Supported States

**Process States (Info):**
- `creating` - Function sedang dibuat
- `booting` - Function sedang boot
- `deploying` - Function sedang di-deploy
- `development` - Function dalam mode development
- `starting` - Function sedang start

**Active States (Success):**
- `ready` - Function siap digunakan ✅
- `active` - Function aktif
- `running` - Function berjalan

**Warning States (Warning):**
- `stopping` - Function sedang dihentikan
- `stopped` - Function telah berhenti
- `paused` - Function di-pause
- `inactive` - Function tidak aktif

**Error States (Error):**
- `error` - Function mengalami error ⚠️
- `failed` - Function deployment gagal ⚠️
- `crashed` - Function crash ⚠️

**Fallback:**
Jika state tidak terdefinisi, sistem akan mengirim notifikasi generic:
- Title: "Function State Changed"
- Message: "Function {name} state changed to: {state}"
- Type: Info

## Error State Handling (IMPORTANT!)

### Problem yang Diperbaiki
Function yang langsung error tanpa melalui state transisi (creating/deploying) akan tetap mendapat notifikasi.

### How It Works
1. **Normal Flow**: Creating → Error
   - Monitor deteksi state change dari 'creating' ke 'error'
   - Notification terkirim ✅

2. **Quick Error Flow**: Langsung Error (tanpa creating)
   - Monitor mount dengan state 'error' (previousState = null)
   - Special handling: Karena state adalah error state, notification tetap terkirim ✅
   - Polling langsung stop

3. **Old Function dengan Error State**
   - Function yang error > 10 menit yang lalu: Tidak di-monitor (expected)
   - Function yang error < 10 menit yang lalu: Masih di-monitor untuk catch state transitions

## Notification Features

### Notification Properties
- **Title**: Short description of the event
- **Message**: Detailed message with function name and state
- **Type**: `info`, `success`, `warning`, `error`
- **Action URL**: Link to function detail page (`/functions/{id}`)
- **Resource Type**: `function`
- **Resource ID**: Function ID
- **Timestamp**: Auto-generated
- **Read Status**: Unread by default

### User Actions
1. **Click Notification**: Mark as read & navigate to function detail
2. **Mark as Read**: Manual mark single notification
3. **Mark All as Read**: Mark all notifications as read
4. **Remove Notification**: Delete single notification
5. **Clear All**: Remove all notifications

## Polling Strategy

### Active Monitoring
- Functions in **non-terminal states** are polled every **3 seconds**
- Functions in **error states** are polled once then stop (notification sent immediately)
- Stops polling when state reaches stable terminal state (ready, stopped, inactive)

### Background Monitoring
- Main functions list refreshes every **5 seconds** to detect new active functions
- Efficient: Only monitors functions that need monitoring
- Recently created functions (< 10 minutes) always monitored

### Terminal States Detection
Polling stops when function reaches:
- Successfully deployed (`ready`)
- Manually stopped (`stopped`, `inactive`)
- Error states (`failed`, `error`, `crashed`) - after sending notification

## Performance Considerations

1. **Selective Polling**: Only polls functions in non-terminal states OR recently created
2. **Auto-cleanup**: Stops monitoring when function reaches stable state OR ages out (> 10 minutes)
3. **Batched Updates**: Uses React Query for efficient data fetching
4. **Persistent Storage**: Notifications stored in localStorage (max 50)
5. **Smart Detection**: Only sends notification when state actually changes
6. **Error State Optimization**: Error states polled once, then polling stops

## Examples

### Example 1: Successful Deployment
```
1. Create Function
   └─> Notifikasi: "Function Created" (info)

2. State: creating
   └─> Monitor mount, set initial state, no notification yet

3. State: creating → deploying
   └─> Notifikasi: "Function Deploying" (info)

4. State: deploying → ready
   └─> Notifikasi: "Function Ready" (success)
   └─> Polling STOP ✅
```

### Example 2: Quick Error (< 1 second)
```
1. Create Function
   └─> Notifikasi: "Function Created" (info)

2. State: LANGSUNG error (tanpa creating)
   └─> Monitor mount dengan state 'error'
   └─> Special handling: Notifikasi "Function Error" (error) ✅
   └─> Polling STOP ✅
```

### Example 3: Normal Error Flow
```
1. Create Function
   └─> Notifikasi: "Function Created" (info)

2. State: creating
   └─> Monitor mount, set initial state

3. State: creating → error
   └─> Notifikasi: "Function Error" (error)
   └─> Polling STOP ✅
```

### Example 4: Manual Stop
```
1. State: ready → stopping
   └─> Notifikasi: "Function Stopping" (warning)

2. State: stopping → stopped
   └─> Notifikasi: "Function Stopped" (warning)
   └─> Polling STOP ✅
```

## Future Enhancements

- [ ] WebSocket integration for real-time updates (eliminate polling)
- [ ] Sound/browser notifications for important events
- [ ] Notification filtering by type/resource
- [ ] Notification search
- [ ] Export notification history
- [ ] Email/Slack integration for critical events
- [ ] Group notifications by function
- [ ] Notification preferences (mute specific states)
- [ ] Retry notification for failed functions

## Testing

To test the notification system:

1. **Test Normal Flow**
   - Create a function that deploys successfully
   - Verify notifications: Created → Creating → Deploying → Ready

2. **Test Quick Error**
   - Create a function with invalid configuration (to force immediate error)
   - Verify notification "Function Error" appears even if error happens immediately

3. **Test State Transitions**
   - Stop a running function → verify stopping/stopped notifications
   - Create function that fails → verify failed notification

4. **Test Notification Actions**
   - Click notification → should navigate to function detail
   - Mark as read → notification should appear read
   - Delete notification → notification should disappear
   - Clear all → all notifications should be removed

5. **Test Edge Cases**
   - Refresh page with function in error state (created < 10 min ago) → still monitored
   - Refresh page with old function (created > 10 min ago) → not monitored
   - Create multiple functions rapidly → all should get notifications

## Troubleshooting

### Error state notifications not appearing
- **Check**: Is function newly created (< 10 minutes)?
  - Yes: Should be monitored and notification sent
  - No: Expected behavior (old functions not retroactively notified)
- **Check**: Browser console for errors
- **Check**: localStorage for notification storage
- **Verify**: Provider is rendering monitors for functions with error states

### Notifications not appearing
- Check browser localStorage: `localStorage.getItem('notification-storage')`
- Verify `FunctionStateMonitorProvider` is rendered in layout
- Check browser console for errors
- Ensure function state is actually changing

### Polling not stopping
- Verify function state is reaching terminal state
- Check React Query devtools for active queries
- Look for query key: `['function-state-monitor', functionId]`

### Duplicate notifications
- Clear localStorage: `localStorage.removeItem('notification-storage')`
- Check for multiple `FunctionStateMonitorProvider` instances
- Verify `previousStateRef` is working correctly

### Missing state notifications
- Check if state is defined in `stateMessages` object
- Fallback will handle undefined states with generic message
- Add custom state to `stateMessages` if needed

## Debugging

Enable React Query devtools in development:

```typescript
// In your app
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'

<ReactQueryDevtools initialIsOpen={false} />
```

Monitor active queries for function state monitors to see polling activity.

Check console logs for notification events (add temporary logging if needed):
```typescript
// In notification store
addNotification: (notification) => {
  console.log('Adding notification:', notification)
  // ... rest of code
}
```
