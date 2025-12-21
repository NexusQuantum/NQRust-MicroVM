+++
title = "Manage Functions"
description = "Invoke, update, monitor, and delete serverless functions"
weight = 42
date = 2025-12-18
+++

Learn how to manage your serverless functions through the web interface.

---

## Accessing Functions

Navigate to **Functions** page from the sidebar to see all your functions:

![Image: Functions list page](/images/functions/functions-page.png)

The Functions page provides:
- **New Function** button - Create new functions
- **Playground** button - Experiment without creating
- **Refresh** button - Update function list
- **Search bar** - Find functions by name or ID
- **Runtime filter** - Filter by language (Python, JavaScript, TypeScript)
- **State filter** - Filter by Ready, Creating, Deploying, Error
- **Function table** - List of all functions with details and actions

---

## Function States

Functions can be in several states:

![Image: Function state badges](/images/functions/function-states.png)

| State | Description | Actions Available |
|-------|-------------|-------------------|
| **Ready** | Function is deployed and ready | Invoke, View Logs, Delete |
| **Creating** | Function is being created | None (wait) |
| **Deploying** | microVM is being provisioned | None (wait) |
| **Error** | Function failed to deploy | View Logs, Delete |

**State transitions**:
- Creating → Deploying → Ready
- Any state → Error (if something fails)

---

## Filtering and Searching Functions

### Search by Name or ID

Use the search bar to quickly find functions:

![Image: Search bar in Functions page](/images/functions/search-bar.png)

- Type function name (e.g., "hello")
- Or type function ID
- Results filter instantly as you type
- Search is case-insensitive

---

### Filter by Runtime

Filter by programming language:

![Image: Runtime filter dropdown](/images/functions/runtime-filter.png)

**Options**:
- **All Languages** - Show all functions (default)
- **Python** - Only Python functions
- **JavaScript (Bun)** - Only JavaScript functions
- **TypeScript (Bun)** - Only TypeScript functions

---

### Filter by State

Filter by function state:

![Image: State filter dropdown](/images/functions/state-filter.png)

**Options**:
- **All States** - Show all functions (default)
- **Ready** - Only ready functions
- **Creating** - Functions being created
- **Deploying** - Functions being deployed
- **Error** - Failed functions

**Tip**: Combine search and filters for precise results.

---

## Function Table Information

The Functions table shows detailed information:

![Image: Function table with all columns](/images/functions/function-table.png)

### Columns Explained

1. **Name** - Function name (click to open detail page)
2. **Language** - Runtime badge (Python, JavaScript, TypeScript)
3. **State** - Current status with colored badge
4. **Last Invoked** - Relative time (e.g., "2 hours ago") or "Never"
5. **24h Invocations** - Number of invocations in last 24 hours
6. **Guest IP** - IP address and port of function microVM
7. **CPU** - vCPU count (e.g., "1 vCPU")
8. **Memory** - Allocated memory in MB (e.g., "512 MB")
9. **Owner** - Who created the function:
   - **"You"** (green) - Your function
   - **"Other User"** - Another user's function
   - **"System"** - System-created function
10. **Actions** - Action buttons (Invoke, Logs, Delete)

### Pagination

If you have more than 10 functions:

![Image: Pagination controls](/images/functions/pagination.png)

- **10 functions per page**
- Click page numbers to navigate
- Use Previous/Next arrows

---

## Invoking Functions

### Invoke from Functions List

To test a function, click the **Invoke** button (▶ Play icon):

![Image: Invoke button on function row](/images/functions/invoke-button.png)

**Steps**:
1. Locate your function in the table
2. In **Actions** column, click **Invoke** button
3. Invoke dialog opens

---

### Invoke Dialog

The invoke dialog lets you test your function with custom input:

![Image: Invoke function dialog](/images/functions/invoke-dialog.png)

#### 1. Enter JSON Payload

Write or paste JSON payload in the Monaco editor:

![Image: JSON payload editor in invoke dialog](/images/functions/invoke-payload-editor.png)

**Example payloads**:
```json
{
  "name": "Alice",
  "age": 30
}
```

```json
{
  "operation": "add",
  "a": 10,
  "b": 5
}
```

#### 2. Validate JSON

The editor validates JSON in real-time:

![Image: JSON valid indicator](/images/functions/json-valid.png)

✅ **"JSON valid"** (green) - Ready to invoke

![Image: JSON invalid error](/images/functions/json-invalid.png)

❌ **"Error: Unexpected token..."** (red) - Fix JSON first

**Tip**: Click **Format JSON** button to auto-format.

#### 3. Click Invoke

Click the **Invoke** button to execute:

![Image: Invoke button in dialog](/images/functions/invoke-button-dialog.png)

The function will execute and show response:

![Image: Function response in dialog](/images/functions/invoke-response.png)

#### 4. View Response

The response panel shows:

**Response body**:
```json
{
  "statusCode": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"result\": 15}"
}
```

**Response actions**:
- **Copy** - Copy response to clipboard
- **Clear** - Clear response panel

### Close Dialog

Click **Cancel** to close the invoke dialog.

**Note**: The payload is saved per-function, so reopening will show your last input.

---

## Viewing Function Logs

To debug or monitor function executions:

### Access Logs

Click the **Logs** button (📄 FileText icon) in the Actions column:

![Image: Logs button on function row](/images/functions/logs-button.png)

This opens the Function Detail page on the Logs tab.

See [View Logs](logs/) for detailed logging information.

---

## Updating Functions

To modify an existing function:

### 1. Open Function Detail Page

Click the **function name** in the table:

![Image: Function name link](/images/functions/function-name-link.png)

### 2. Edit Function

The function editor opens with current code:

![Image: Function editor in update mode](/images/functions/function-editor-update.png)

### 3. Make Changes

You can update:
- ✅ **Name** - Change function name
- ✅ **Code** - Edit function code
- ✅ **Handler** - Change entry point
- ✅ **Timeout** - Adjust timeout seconds
- ✅ **Memory** - Change memory allocation

**Cannot change**:
- ❌ **Runtime** - Cannot change after creation (delete and recreate instead)
- ❌ **vCPU** - Fixed at creation time

### 4. Test Changes

Use the **Run Test** section to test your changes:

![Image: Test section in update mode](/images/functions/test-update.png)

### 5. Save Updates

Click **Save** to deploy changes:

![Image: Save button highlighted](/images/functions/save-update-button.png)

The function will be redeployed with new code.

**Note**: Function may be briefly unavailable during update (2-5 seconds).

---

## Deleting Functions

**⚠️ Warning**: Deletion is **permanent** and **cannot be undone**!

### Delete a Function

From the Functions list page:

![Image: Delete button on function row](/images/functions/delete-button.png)

**Steps**:
1. In the **Actions** column, click the **Delete** button (🗑️ Trash icon)
2. A confirmation dialog will appear:

![Image: Delete confirmation dialog](/images/functions/delete-confirm.png)

3. Click **Delete** to confirm

The function will be permanently removed.

### What Gets Deleted

When you delete a function:

- ✅ **Function code** - All code removed
- ✅ **Configuration** - Settings deleted
- ✅ **microVM** - VM destroyed and resources freed
- ✅ **Logs** - Execution logs removed
- ✅ **Invocation history** - Stats cleared

**Time**: Usually instant (< 1 second)

### Success Notification

After deletion, you'll see a success message:

![Image: Function deleted success notification](/images/functions/delete-success.png)

**"Function Deleted - [function-name] has been deleted successfully"**

The function will be removed from the Functions list.

---

## Function Detail Page

Click a function name to open the detail page:

![Image: Function detail page overview](/images/functions/function-detail-page.png)

### Overview Tab

Shows function information:

![Image: Function overview tab](/images/functions/function-overview-tab.png)

**Information displayed**:
- Function name and ID
- Runtime and state badge
- vCPU, Memory, Timeout
- Guest IP address
- Creation date
- Last invoked time
- Invocation count (24h, 7d, 30d)

**Actions available**:
- **Invoke** - Test function
- **Edit** - Modify function
- **Delete** - Remove function

---

### Logs Tab

View execution logs and errors:

![Image: Function logs tab](/images/functions/function-logs-tab.png)

See [View Logs](logs/) for details.

---

### Code Tab (if available)

View current function code:

![Image: Function code tab](/images/functions/function-code-tab.png)

**Features**:
- Read-only code viewer
- Syntax highlighting
- Copy code to clipboard

**To edit**: Click **Edit** button or switch to edit mode.

---

## Refresh Function List

To update the function list with latest data:

Click the **Refresh** button in the Functions page header:

![Image: Refresh button](/images/functions/refresh-button.png)

**When to refresh**:
- After creating a function (to see new state)
- If states seem stale
- To update invocation counts
- After another user makes changes

**Note**: The list auto-refreshes periodically, but you can manual refresh for instant updates.

---

## Monitoring Functions

### Invocation Metrics

Track how often your functions are called:

![Image: Invocation count columns](/images/functions/invocation-metrics.png)

**Metrics shown**:
- **Last Invoked** - When function was last called
- **24h Invocations** - Calls in last 24 hours
- **7d Invocations** - Calls in last 7 days (if shown)
- **30d Invocations** - Calls in last 30 days (if shown)

**Use cases**:
- Identify popular functions
- Detect unused functions (for cleanup)
- Monitor traffic patterns
- Track adoption

---

### Function Health

Monitor function state and errors:

**Healthy function**:
- ✅ State: **Ready** (green)
- ✅ Invocations succeeding
- ✅ No errors in logs

**Unhealthy function**:
- ❌ State: **Error** (red)
- ❌ Invocations failing
- ❌ Errors in logs

**Actions for unhealthy functions**:
1. Click **Logs** to view error messages
2. Identify the issue (syntax error, timeout, etc.)
3. Click function name to **Edit**
4. Fix the issue
5. **Save** to redeploy
6. **Invoke** to test

---

## Troubleshooting

### Issue: Cannot Invoke Function

**Symptoms**:
- Invoke button doesn't work
- Error message appears
- Timeout error

**Solutions**:

1. **Check function state**:
   - State must be **"Ready"** to invoke
   - If "Creating" or "Deploying", wait for it to finish
   - If "Error", check logs and fix issues

2. **Check JSON payload**:
   - Must be valid JSON
   - Check for syntax errors
   - Click "Format JSON" to verify

3. **Check network**:
   - Ensure function microVM is online
   - Check **Guest IP** is assigned
   - Go to **Hosts** page, verify agent is online

4. **Try again**:
   - Refresh the page
   - Try invoking again

---

### Issue: Function Stuck in "Deploying"

**Symptoms**:
- State shows "Deploying" for more than 1 minute
- Never changes to "Ready"

**Solutions**:

1. **Wait longer** - Initial deploy can take up to 2 minutes
2. **Refresh page** - State may be stale
3. **Check logs**:
   - Click **Logs** button
   - Look for deployment errors
4. **Check resources**:
   - Go to **Hosts** page
   - Verify sufficient CPU/memory available
5. **Delete and recreate**:
   - If stuck for >5 minutes, delete function
   - Create a new one with same code

---

### Issue: Invocation Returns Error

**Symptoms**:
- Invoke succeeds but returns error in response
- `statusCode: 500` or other error codes

**Solutions**:

1. **Check response body**:
   ```json
   {
     "statusCode": 500,
     "body": "{\"error\": \"...\"}"
   }
   ```

2. **View function logs**:
   - Click **Logs** button
   - Look for error messages
   - Identify the issue (e.g., undefined variable)

3. **Common issues**:
   - **Missing event fields**: Check payload has required keys
   - **Type errors**: Ensure data types match expectations
   - **Timeout**: Increase timeout or optimize code
   - **Syntax errors**: Review code for typos

4. **Fix and test**:
   - Edit function
   - Fix the issue
   - Save and invoke again

---

### Issue: Cannot Delete Function

**Symptoms**:
- Delete button doesn't respond
- Error message appears

**Solutions**:

1. **Check ownership**:
   - You can only delete functions you created
   - Or if you're an admin

2. **Try from detail page**:
   - Click function name
   - Try deleting from detail page

3. **Refresh and retry**:
   - Refresh the page
   - Try delete again

4. **Contact administrator**:
   - If issue persists

---

## Best Practices

### Function Management

✅ **Organize functions logically**:
- Use descriptive names
- Group related functions with prefixes
  - `auth-login`, `auth-logout`, `auth-verify`
  - `payment-create`, `payment-verify`, `payment-refund`
- Add comments in code explaining purpose

---

### Monitoring and Maintenance

✅ **Regular monitoring**:
- Check invocation counts weekly
- Review error logs regularly
- Delete unused functions
- Update function code when needed

✅ **Clean up unused functions**:
- Filter by "Last Invoked"
- Delete functions not used in 30+ days
- Keep codebase clean

---

### Testing Before Deploy

✅ **Always test before saving**:
- Use **Run Test** in editor
- Test with multiple payloads
- Verify error handling
- Check edge cases

✅ **Test after updates**:
- After saving changes, invoke from list
- Verify response is correct
- Check logs for errors

---

### Resource Optimization

✅ **Right-size functions**:
- Start with minimum resources
- Monitor invocation times in logs
- Increase CPU/memory only if needed
- Over-allocation = higher costs

✅ **Optimize timeout**:
- Set realistic timeout values
- Too short = unnecessary failures
- Too long = wasted resources on errors

---

## Quick Reference

### Function Actions Summary

| Action | Button | Description |
|--------|--------|-------------|
| **Invoke** | ▶ Play | Test function with custom payload |
| **Logs** | 📄 FileText | View execution logs and errors |
| **Delete** | 🗑️ Trash | Permanently remove function |
| **Edit** | Click name | Open editor to modify function |

### Common Workflows

**Daily monitoring**:
1. Open Functions page
2. Check states - all should be "Ready"
3. Review invocation counts
4. Check logs for errors

**Updating a function**:
1. Click function name
2. Edit code
3. Run Test to verify
4. Click Save
5. Invoke from list to confirm

**Debugging errors**:
1. Click Logs button
2. Identify error in logs
3. Click function name to Edit
4. Fix issue
5. Save and test

---

## Next Steps

Now that you know how to manage functions:

- **[View Logs](logs/)** - Learn to debug with logs
- **[Playground](playground/)** - Experiment with new ideas
- **[Create a Function](create-function/)** - Review creation steps

---

## Performance Tips

### Cold Start Optimization

**Cold starts** occur on first invocation or after idle period:

✅ **Minimize cold starts**:
- Keep functions warm with periodic invocations
- Use cron job to invoke every 5 minutes
- Minimize code size (faster load time)
- Avoid heavy dependencies

---

### Warm Invocation Optimization

**Warm invocations** reuse existing microVM:

✅ **Optimize warm performance**:
- Cache expensive operations (DB connections, API clients)
- Initialize once in global scope
- Reuse across invocations
- Minimize per-invocation overhead

**Example** (Python):
```python
# ✅ Good - Initialize once
import requests
session = requests.Session()  # Global scope

def handler(event):
    # Reuse session across invocations
    response = session.get(event["url"])
    return {"statusCode": 200, "body": response.text}

# ❌ Bad - Initialize every time
def handler(event):
    import requests
    session = requests.Session()  # Created every invocation!
    response = session.get(event["url"])
    return {"statusCode": 200, "body": response.text}
```
