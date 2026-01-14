# Users Documentation Images

This directory contains screenshots and images for the Users documentation.

## Required Images

### Navigation & Page Layout
- `nav-users.png` - Sidebar navigation showing Users menu item
- `page-layout.png` - Full Users page showing header and user table
- `header.png` - Users page header with stats

### User Table
- `table.png` - User table showing columns and data
- `table-explained.png` - Annotated table explaining each column
- `search-users.png` - Search box with example search
- `role-filter.png` - Role filter dropdown open
- `search-box.png` - Search box closeup
- `pagination.png` - Pagination controls
- `you-badge.png` - "You" badge next to current user

### Create User
- `create-user-button.png` - Create User button in header
- `create-user-dialog.png` - Full create user dialog
- `user-form-fields.png` - Form fields (username, password, role)
- `create-submit-button.png` - Create button in dialog

### Edit User
- `edit-button.png` - Edit (pencil) icon in table
- `edit-user-dialog.png` - Full edit user dialog
- `save-button.png` - Save button in edit dialog

### Delete User
- `delete-button.png` - Delete (trash) icon in table
- `delete-confirm.png` - Delete confirmation dialog
- `delete-disabled.png` - Disabled delete button for current user
- `user-to-delete.png` - User row with delete action highlighted

### Filters
- `filters.png` - Search and role filter together

### User Actions
- `user-actions.png` - Actions column showing edit/delete buttons

## Image Guidelines

### Format & Size
- **Format**: PNG (preferred) or JPG
- **Width**: Maximum 1920px, optimize for web
- **Quality**: High enough to show UI elements clearly

### Content Guidelines
- **Theme**: Match the UI theme (dark/light as configured)
- **Privacy**: Remove or blur any sensitive data
- **Annotations**: Use red boxes/arrows to highlight important elements
- **Consistency**: Same browser zoom level for all screenshots

### Taking Screenshots

1. **Full Page**: Use browser's full page screenshot
2. **Specific Element**: Crop to show only relevant component
3. **Dialog**: Capture the modal/dialog with backdrop dimmed
4. **Highlighting**: Use red rectangle/arrow for important buttons

## Screenshot Checklist

### Users Page Screenshots
- [ ] Navigation showing Users menu item (sidebar)
- [ ] Full page with header and table
- [ ] Header section closeup (title, stats)
- [ ] Table with multiple users (at least 3)
- [ ] Table showing different roles (Admin, User, Viewer)
- [ ] Table showing "You" badge on current user
- [ ] Search box with typed text
- [ ] Role filter dropdown open
- [ ] Pagination controls (if enough users)

### Create User Screenshots
- [ ] Create User button
- [ ] Create dialog opened
- [ ] Form with all fields visible
- [ ] Role dropdown open
- [ ] Create button

### Edit User Screenshots
- [ ] Edit button (pencil icon)
- [ ] Edit dialog opened
- [ ] Form with pre-filled data
- [ ] Save button

### Delete User Screenshots
- [ ] Delete button (trash icon)
- [ ] Delete confirmation dialog
- [ ] Disabled delete button for current user

## Placeholder Images

Until real screenshots are added, the documentation will work but images won't display. Hugo will show broken image links in development mode.

## Adding Images

1. Take screenshot of the UI element
2. Save with appropriate filename (see list above)
3. Place in this directory
4. No code changes needed - documentation already references these images

## Annotating Screenshots

For better user guidance, consider adding:
- Red rectangles around buttons to click
- Arrows pointing to important elements
- Text callouts for key information
- Numbers for sequential steps

Tools for annotations:
- macOS Preview
- Windows Snipping Tool
- GIMP / Photoshop
- draw.io for diagrams
