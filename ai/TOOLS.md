# AlphaHuman Tools

This document lists all available tools that AlphaHuman can use to interact with external services and perform actions. Tools are organized by integration and automatically updated when the app loads.

## Overview

AlphaHuman has access to **25 tools** across **1 integrations**.

**Quick Statistics:**

- **Notion**: 25 tools

## Available Tools

### Notion Tools

This skill provides 25 tools for notion integration.

#### append-blocks

**Description**: Append child blocks to a page or block. Supports various block types.

**Parameters**:

- **block_id** (string) **(required)**: The parent page or block ID
- **blocks** (string) **(required)**: JSON string of blocks array. Example: [{"type":"paragraph","paragraph":{"rich_text":[{"text":{"content":"Hello"}}]}}]

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "append-blocks",
  "parameters": { "block_id": "example_block_id", "blocks": "example_blocks" }
}
```

---

#### append-text

**Description**: Append text content to a page or block. Use the page id (or block_id) from list-all-pages or get-page. Creates paragraph blocks with the given text.

**Parameters**:

- **block_id** (string): The page or block ID to append to (use page id from list-all-pages)
- **content** (string): Alias for text — the content to append to the page
- **page_id** (string): Alias for block_id when appending to a page (same as block_id)
- **text** (string) **(required)**: The text to append (required). Pass the exact content to add to the page.

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "append-text",
  "parameters": {
    "block_id": "example_block_id",
    "content": "example_content",
    "page_id": "example_page_id",
    "text": "example_text"
  }
}
```

---

#### create-comment

**Description**: Create a comment on a page or block, or reply to a discussion. Provide either page_id (new comment on page) or discussion_id (reply). Requires Notion integration to have insert comment capability.

**Parameters**:

- **block_id** (string): Block ID to comment on (optional, use instead of page_id)
- **discussion_id** (string): Discussion ID to reply to an existing thread (use instead of page_id)
- **page_id** (string): Page ID to create a comment on (new discussion)
- **text** (string) **(required)**: Comment text content

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "create-comment",
  "parameters": {
    "block_id": "example_block_id",
    "discussion_id": "example_discussion_id",
    "page_id": "example_page_id",
    "text": "example_text"
  }
}
```

---

#### create-database

**Description**: Create a new database in Notion. Specify parent page_id and title. Optionally provide properties schema as JSON.

**Parameters**:

- **parent_page_id** (string) **(required)**: Parent page ID where the database will be created
- **properties** (string): JSON string of properties schema. Example: {"Name":{"title":{}},"Status":{"select":{"options":[{"name":"Todo"},{"name":"Done"}]}}}
- **title** (string) **(required)**: Database title

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "create-database",
  "parameters": {
    "parent_page_id": "example_parent_page_id",
    "properties": "example_properties",
    "title": "example_title"
  }
}
```

---

#### create-page

**Description**: Create a new page in Notion. Parent can be another page or a database. For database parents, properties must match the database schema.

**Parameters**:

- **content** (string): Initial text content (creates a paragraph block)
- **parent_id** (string) **(required)**: Parent page ID or database ID
- **parent_type** (string): Type of parent (default: page_id)
- **properties** (string): JSON string of additional properties (for database pages)
- **title** (string) **(required)**: Page title

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "create-page",
  "parameters": {
    "content": "example_content",
    "parent_id": "example_parent_id",
    "parent_type": "example_parent_type",
    "properties": "example_properties",
    "title": "example_title"
  }
}
```

---

#### delete-block

**Description**: Delete a block. Permanently removes the block from Notion.

**Parameters**:

- **block_id** (string) **(required)**: The block ID to delete

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "delete-block", "parameters": { "block_id": "example_block_id" } }
```

---

#### delete-page

**Description**: Delete (archive) a page. Archived pages can be restored from Notion's trash.

**Parameters**:

- **page_id** (string) **(required)**: The page ID to delete/archive

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "delete-page", "parameters": { "page_id": "example_page_id" } }
```

---

#### get-block

**Description**: Get a block by its ID. Returns the block's type and content.

**Parameters**:

- **block_id** (string) **(required)**: The block ID

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "get-block", "parameters": { "block_id": "example_block_id" } }
```

---

#### get-block-children

**Description**: Get the children blocks of a block or page.

**Parameters**:

- **block_id** (string) **(required)**: The parent block or page ID
- **page_size** (number): Number of blocks (default 50, max 100)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "get-block-children", "parameters": { "block_id": "example_block_id", "page_size": 10 } }
```

---

#### get-database

**Description**: Get a database's schema and metadata. Shows all properties and their types.

**Parameters**:

- **database_id** (string) **(required)**: The database ID

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "get-database", "parameters": { "database_id": "example_database_id" } }
```

---

#### get-page

**Description**: Get a page's metadata and properties by its ID. Use notion-get-page-content to get the actual content/blocks.

**Parameters**:

- **page_id** (string) **(required)**: The page ID (UUID format, with or without dashes)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "get-page", "parameters": { "page_id": "example_page_id" } }
```

---

#### get-page-content

**Description**: Get the content blocks of a page. Returns the text and structure of the page. Use recursive=true to also get nested blocks.

**Parameters**:

- **page_id** (string) **(required)**: The page ID to get content from
- **page_size** (number): Number of blocks to return (default 50, max 100)
- **recursive** (string): Whether to fetch nested blocks (default: false)

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "get-page-content",
  "parameters": { "page_id": "example_page_id", "page_size": 10, "recursive": "example_recursive" }
}
```

---

#### get-user

**Description**: Get a user by their ID.

**Parameters**:

- **user_id** (string) **(required)**: The user ID

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "get-user", "parameters": { "user_id": "example_user_id" } }
```

---

#### list-all-databases

**Description**: List all databases in the workspace that the integration has access to.

**Parameters**:

- **page_size** (number): Number of results (default 20, max 100)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "list-all-databases", "parameters": { "page_size": 10 } }
```

---

#### list-all-pages

**Description**: List pages in the workspace (from last sync). Returns synced pages; run a sync in Settings to refresh.

**Parameters**:

- **page_size** (number): Number of results to return (default 20, max 100)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "list-all-pages", "parameters": { "page_size": 10 } }
```

---

#### list-comments

**Description**: List comments on a block or page.

**Parameters**:

- **block_id** (string) **(required)**: Block or page ID to get comments for
- **page_size** (number): Number of results (default 20, max 100)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "list-comments", "parameters": { "block_id": "example_block_id", "page_size": 10 } }
```

---

#### list-users

**Description**: List all users in the workspace that the integration can see.

**Parameters**:

- **page_size** (number): Number of results (default 20, max 100)

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "list-users", "parameters": { "page_size": 10 } }
```

---

#### query-database

**Description**: Query a database with optional filters and sorts. Returns database rows/pages. Automatically handles API version compatibility.

**Parameters**:

- **database_id** (string) **(required)**: The database ID to query. Can be either a legacy database ID or a new data source ID - the tool will handle both automatically
- **filter** (string): JSON string of filter object (Notion filter syntax)
- **page_size** (number): Number of results (default 20, max 100)
- **sorts** (string): JSON string of sorts array (Notion sort syntax)

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "query-database",
  "parameters": {
    "database_id": "example_database_id",
    "filter": "example_filter",
    "page_size": 10,
    "sorts": "example_sorts"
  }
}
```

---

#### search

**Description**: Search for pages and databases in your Notion workspace. Supports query, filter by object type (page or database), and sort by last_edited_time.

**Parameters**:

- **filter** (string): Filter results by type: page or database
- **page_size** (number): Number of results to return (default 20, max 100)
- **query** (string): Search query (optional, returns recent if empty)
- **sort_direction** (string): Sort direction (default: descending by last_edited_time)

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "search",
  "parameters": {
    "filter": "example_filter",
    "page_size": 10,
    "query": "example_query",
    "sort_direction": "example_sort_direction"
  }
}
```

---

#### summarize-pages

**Description**: AI summarization of Notion pages is now handled by the backend server. Synced page content is submitted to the server which runs summarization.

**Parameters**: _None_

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "summarize-pages", "parameters": {} }
```

---

#### sync-now

**Description**: Trigger an immediate Notion sync to refresh local data. Returns sync results including counts of synced pages and databases.

**Parameters**: _None_

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "sync-now", "parameters": {} }
```

---

#### sync-status

**Description**: Get the current Notion sync status including last sync time, total synced pages/databases, sync progress, and any errors.

**Parameters**: _None_

**Usage Context**: Available in all environments

**Example**:

```json
{ "tool": "sync-status", "parameters": {} }
```

---

#### update-block

**Description**: Update a block's content. The structure depends on the block type.

**Parameters**:

- **archived** (string): Set to true to archive the block
- **block_id** (string) **(required)**: The block ID to update
- **content** (string): JSON string of the block type content. Example for paragraph: {"paragraph":{"rich_text":[{"text":{"content":"Updated text"}}]}}

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "update-block",
  "parameters": {
    "archived": "example_archived",
    "block_id": "example_block_id",
    "content": "example_content"
  }
}
```

---

#### update-database

**Description**: Update a database's title or properties schema.

**Parameters**:

- **database_id** (string) **(required)**: The database ID to update
- **properties** (string): JSON string of properties to add or update
- **title** (string): New title (optional)

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "update-database",
  "parameters": {
    "database_id": "example_database_id",
    "properties": "example_properties",
    "title": "example_title"
  }
}
```

---

#### update-page

**Description**: Update a page's properties. Can update title and other properties. Use notion-append-text to add content blocks.

**Parameters**:

- **archived** (string): Set to true to archive the page
- **page_id** (string) **(required)**: The page ID to update
- **properties** (string): JSON string of properties to update
- **title** (string): New title (optional)

**Usage Context**: Available in all environments

**Example**:

```json
{
  "tool": "update-page",
  "parameters": {
    "archived": "example_archived",
    "page_id": "example_page_id",
    "properties": "example_properties",
    "title": "example_title"
  }
}
```

---

## Tool Usage Guidelines

### Authentication

- All tools require proper authentication setup through the Skills system
- OAuth credentials are managed securely and refreshed automatically
- API keys are stored encrypted in the application keychain

### Rate Limiting

- Tools automatically respect API rate limits of external services
- Intelligent retry logic handles temporary failures with exponential backoff

### Error Handling

- All tools return structured error responses with detailed information
- Network failures trigger automatic retry with configurable attempts

---

**Tool Statistics**

- Total Tools: 25
- Active Skills: 1
- Last Updated: 2026-03-17T17:02:14.087Z

_This file was automatically generated when the app loaded._
_Tools are discovered from the running V8 skills runtime._
