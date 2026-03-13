# Notion

A Rust library for interacting with the Notion API with permission-based access control.

This library provides a simple interface to work with Notion pages, databases, and data sources while enforcing configurable permissions.

## Installation

### Cargo

```
cargo install --path . --features mcp
```

It could also use as the dependency in your project. Add to your `Cargo.toml`:

```toml
[dependencies]
notion = { git = "https://github.com/mikojs/notion" }
```

### Nix

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    miko-notion.url = "github:mikojs/notion";
  };

  outputs = { nixpkgs, miko-notion, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      ...
      modules = [
        ({ pkgs, ... }: {
          nixpkgs.overlays = [ miko-notion.overlays.default ];
          environment.systemPackages = [ pkgs.miko-notion ];
        })
      ];
    };
  };
}
```

## Configuration

### Environment Variables

- `NOTION_TOKEN`: Your Notion API token (required)
- `NOTION_CONFIG`: Path to config file (optional, defaults to `~/.config/notion.json`)

Follow [this guide](https://www.notion.com/help/create-integrations-with-the-notion-api) to create a Notion API token.

### Config File

Create a config file to define permissions for databases and data sources:

```json
[
  {
    "id": "your-database-id",
    "name": "my-database",
    "type": "Database",
    "permission": ["Get", "Add", "Update"]
  },
  {
    "id": "your-data-source-id",
    "name": "my-data-source",
    "type": "DataSource",
    "permission": ["Get"]
  }
]
```

## Usage as a MCP server

```json
{
  "mcpServers": {
    "notion": {
      "commands": "notion"
    }
  }
}
```

## Usage as a Library

Follow [this guide](https://developers.notion.com/reference/post-database-query-filter) to create a filter for data sources if you need.

### Initialize Client

```rust
use notion::Notion;

let client = Notion::new()?;
```

### Get Page

```rust
let page = client.get_page("page-id").await?;
```

### Add Page

```rust
use serde_json::json;

let page_data = json!({
    "parent": { "database_id": "your-database-id" },
    "properties": {
        "Name": {
            "title": [{ "text": { "content": "New Page" } }]
        }
    }
});

client.add_page(page_data).await?;
```

### Update Page

```rust
use serde_json::json;

let updates = json!({
    "properties": {
        "Status": {
            "select": { "name": "Done" }
        }
    }
});

client.update_page("page-id", updates).await?;
```

### Get Database

```rust
let database = client.get_database("my-database").await?;
```

### Query Data Sources

```rust
use serde_json::json;

let filter = json!({});
let results = client.get_data_sources("my-data-source", &filter).await?;
```

## Testing

Enable the `test-utils` feature to use `MockNotion` for testing:

```toml
[dev-dependencies]
notion = { git = "https://github.com/mikojs/notion", features = ["test-utils"] }
```

### Example

```rust
use notion::{NotionTrait, mock::MockNotion};
use serde_json::json;

#[tokio::test]
async fn test_get_list() {
    let mock = MockNotion::new();

    mock.mock_get_list(|| vec![]).await;

    let result = mock.get_list();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_page() {
    let mock = MockNotion::new();

    mock.mock_get_page(|| Ok(json!({
        "id": "test-page-id",
        "properties": {}
    }))).await;

    let result = mock.get_page("any-id").await.unwrap();
    assert_eq!(result["id"], "test-page-id");
}
```
