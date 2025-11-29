# Ruby on Rails Integration Guide

This guide explains how to use the Pubky.app data model specifications (`pubky-app-specs`) in a Ruby on Rails application via FFI (Foreign Function Interface).

## Quick Start

The library includes FFI exports in `src/ffi.rs` that allow you to use the models directly from Ruby/Rails.

```ruby
# Example: Create and validate a post
result = PubkyFFI.create_post(content: 'Hello, Pubky!', kind: 'short')
puts "Post ID: #{result[:id]}" if result[:success]

# Example: Generate a file ID
file_result = PubkyFFI.create_file(
  name: 'photo.jpg',
  src: 'pubky://user123/pub/pubky.app/blobs/abc123',
  content_type: 'image/jpeg',
  size: 512_000
)
puts "File ID: #{file_result[:id]}"
```

## Table of Contents

- [Building the Shared Library](#building-the-shared-library)
- [Ruby FFI Setup](#ruby-ffi-setup)
- [Available Functions](#available-functions)
- [Usage Examples](#usage-examples)
- [Integration with Rails](#integration-with-rails)
- [Why Not WASM?](#why-not-wasm)

---

## Building the Shared Library

Build the library as a shared object that can be loaded via FFI:

```bash
# Clone the repository
git clone https://github.com/pubky/pubky-app-specs.git
cd pubky-app-specs

# Build release shared library
cargo build --release

# The shared library will be at:
# Linux:   target/release/libpubky_app_specs.so
# macOS:   target/release/libpubky_app_specs.dylib
# Windows: target/release/pubky_app_specs.dll
```

Copy the shared library to your Rails application:

```bash
mkdir -p your_rails_app/lib/native
cp target/release/libpubky_app_specs.so your_rails_app/lib/native/
```

---

## Ruby FFI Setup

Add the `ffi` gem to your Gemfile:

```ruby
# Gemfile
gem 'ffi', '~> 1.16'
```

Create an initializer to load and wrap the library:

```ruby
# config/initializers/pubky_ffi.rb
require 'ffi'
require 'json'

module PubkyFFI
  extend FFI::Library

  # Load the shared library based on platform
  LIB_NAME = case RbConfig::CONFIG['host_os']
             when /darwin/  then 'libpubky_app_specs.dylib'
             when /linux/   then 'libpubky_app_specs.so'
             when /mswin|mingw/ then 'pubky_app_specs.dll'
             end

  ffi_lib Rails.root.join('lib', 'native', LIB_NAME).to_s

  # Memory management
  attach_function :pubky_free_string, [:pointer], :void

  # Post functions
  attach_function :pubky_create_post, [:string, :string], :pointer
  attach_function :pubky_create_post_full, [:string, :string, :string, :string, :string], :pointer
  attach_function :pubky_validate_post, [:string, :string], :pointer
  attach_function :pubky_generate_timestamp_id, [], :pointer

  # File functions
  attach_function :pubky_create_file, [:string, :string, :string, :size_t], :pointer
  attach_function :pubky_validate_file, [:string, :string], :pointer

  # Bookmark functions
  attach_function :pubky_create_bookmark, [:string], :pointer
  attach_function :pubky_validate_bookmark, [:string, :string], :pointer

  # Tag functions
  attach_function :pubky_create_tag, [:string, :string], :pointer
  attach_function :pubky_validate_tag, [:string, :string], :pointer

  # User functions
  attach_function :pubky_create_user, [:string, :string, :string, :string], :pointer
  attach_function :pubky_create_user_with_links, [:string, :string, :string, :string, :string], :pointer
  attach_function :pubky_validate_user, [:string], :pointer

  # Path helpers
  attach_function :pubky_get_post_path, [:string], :pointer
  attach_function :pubky_get_file_path, [:string], :pointer
  attach_function :pubky_get_bookmark_path, [:string], :pointer
  attach_function :pubky_get_tag_path, [:string], :pointer
  attach_function :pubky_get_user_path, [], :pointer

  class << self
    # Create a post
    def create_post(content:, kind: 'short')
      call_and_parse(:pubky_create_post, content, kind)
    end

    # Create a post with all options
    def create_post_full(content:, kind: 'short', parent: nil, embed_uri: nil, embed_kind: nil)
      call_and_parse(:pubky_create_post_full, content, kind, parent, embed_uri, embed_kind)
    end

    # Validate a post
    def validate_post(json_or_hash, id: nil)
      json_str = json_or_hash.is_a?(String) ? json_or_hash : json_or_hash.to_json
      call_and_parse(:pubky_validate_post, json_str, id)
    end

    # Generate a timestamp-based ID
    def generate_id
      ptr = pubky_generate_timestamp_id
      result = ptr.read_string
      pubky_free_string(ptr)
      result
    end

    # Create a file
    def create_file(name:, src:, content_type:, size:)
      call_and_parse(:pubky_create_file, name, src, content_type, size)
    end

    # Validate a file
    def validate_file(json_or_hash, id: nil)
      json_str = json_or_hash.is_a?(String) ? json_or_hash : json_or_hash.to_json
      call_and_parse(:pubky_validate_file, json_str, id)
    end

    # Create a bookmark
    def create_bookmark(uri:)
      call_and_parse(:pubky_create_bookmark, uri)
    end

    # Validate a bookmark
    def validate_bookmark(json_or_hash, id: nil)
      json_str = json_or_hash.is_a?(String) ? json_or_hash : json_or_hash.to_json
      call_and_parse(:pubky_validate_bookmark, json_str, id)
    end

    # Create a tag
    def create_tag(uri:, label:)
      call_and_parse(:pubky_create_tag, uri, label)
    end

    # Validate a tag
    def validate_tag(json_or_hash, id: nil)
      json_str = json_or_hash.is_a?(String) ? json_or_hash : json_or_hash.to_json
      call_and_parse(:pubky_validate_tag, json_str, id)
    end

    # Create a user profile
    def create_user(name:, bio: nil, image: nil, status: nil)
      call_and_parse(:pubky_create_user, name, bio, image, status)
    end

    # Create a user profile with links
    def create_user_with_links(name:, bio: nil, image: nil, status: nil, links: nil)
      links_json = links&.to_json
      call_and_parse(:pubky_create_user_with_links, name, bio, image, status, links_json)
    end

    # Validate a user profile
    def validate_user(json_or_hash)
      json_str = json_or_hash.is_a?(String) ? json_or_hash : json_or_hash.to_json
      call_and_parse(:pubky_validate_user, json_str)
    end

    # Get paths
    def post_path(id)
      call_and_read(:pubky_get_post_path, id)
    end

    def file_path(id)
      call_and_read(:pubky_get_file_path, id)
    end

    def bookmark_path(id)
      call_and_read(:pubky_get_bookmark_path, id)
    end

    def tag_path(id)
      call_and_read(:pubky_get_tag_path, id)
    end

    def user_path
      ptr = pubky_get_user_path
      result = ptr.read_string
      pubky_free_string(ptr)
      result
    end

    private

    def call_and_parse(method, *args)
      ptr = send(method, *args)
      result = ptr.read_string
      pubky_free_string(ptr)
      JSON.parse(result, symbolize_names: true)
    end

    def call_and_read(method, *args)
      ptr = send(method, *args)
      return nil if ptr.null?
      result = ptr.read_string
      pubky_free_string(ptr)
      result
    end
  end
end
```

---

## Available Functions

### Post Functions

| Function | Description |
|----------|-------------|
| `pubky_create_post(content, kind)` | Create a new post with content and kind |
| `pubky_create_post_full(content, kind, parent, embed_uri, embed_kind)` | Create a post with all optional fields |
| `pubky_validate_post(json, id)` | Validate a post from JSON |
| `pubky_generate_timestamp_id()` | Generate a timestamp-based ID |

### File Functions

| Function | Description |
|----------|-------------|
| `pubky_create_file(name, src, content_type, size)` | Create a new file entry |
| `pubky_validate_file(json, id)` | Validate a file from JSON |

### Bookmark Functions

| Function | Description |
|----------|-------------|
| `pubky_create_bookmark(uri)` | Create a bookmark for a URI |
| `pubky_validate_bookmark(json, id)` | Validate a bookmark from JSON |

### Tag Functions

| Function | Description |
|----------|-------------|
| `pubky_create_tag(uri, label)` | Create a tag for a URI with a label |
| `pubky_validate_tag(json, id)` | Validate a tag from JSON |

### User Functions

| Function | Description |
|----------|-------------|
| `pubky_create_user(name, bio, image, status)` | Create a user profile |
| `pubky_create_user_with_links(name, bio, image, status, links_json)` | Create a user with links |
| `pubky_validate_user(json)` | Validate a user profile from JSON |

### Path Helpers

| Function | Description |
|----------|-------------|
| `pubky_get_post_path(id)` | Get the path for a post |
| `pubky_get_file_path(id)` | Get the path for a file |
| `pubky_get_bookmark_path(id)` | Get the path for a bookmark |
| `pubky_get_tag_path(id)` | Get the path for a tag |
| `pubky_get_user_path()` | Get the path for a user profile |

---

## Usage Examples

### Creating and Validating a Post

```ruby
# Create a short post
result = PubkyFFI.create_post(
  content: 'Hello, Pubky world!',
  kind: 'short'
)

if result[:success]
  puts "Post ID: #{result[:id]}"
  puts "Path: #{result[:path]}"
  puts "Post: #{result[:post]}"
else
  puts "Error: #{result[:error]}"
end

# Create a reply to another post
reply = PubkyFFI.create_post_full(
  content: 'This is a reply!',
  kind: 'short',
  parent: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0'
)

# Validate a post
validation = PubkyFFI.validate_post({
  content: 'Test content',
  kind: 'short'
})
puts "Valid: #{validation[:valid]}"
```

### Working with Files

```ruby
# Create a file entry
file_result = PubkyFFI.create_file(
  name: 'photo.jpg',
  src: 'pubky://user123/pub/pubky.app/blobs/abc123',
  content_type: 'image/jpeg',
  size: 512_000
)

if file_result[:success]
  puts "File ID: #{file_result[:id]}"
  puts "File Path: #{file_result[:path]}"
end
```

### Creating Bookmarks and Tags

```ruby
# Bookmark a post
bookmark = PubkyFFI.create_bookmark(
  uri: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0'
)
puts "Bookmark ID: #{bookmark[:id]}"

# Tag a post
tag = PubkyFFI.create_tag(
  uri: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0',
  label: 'interesting'
)
puts "Tag ID: #{tag[:id]}"
```

### User Profiles

```ruby
# Create a user profile
user = PubkyFFI.create_user(
  name: 'Alice',
  bio: 'Developer and Pubky enthusiast',
  image: 'https://example.com/avatar.png',
  status: 'Building cool stuff'
)
puts "User path: #{user[:path]}"

# Create a user with links
user_with_links = PubkyFFI.create_user_with_links(
  name: 'Alice',
  bio: 'Developer',
  image: nil,
  status: nil,
  links: [
    { title: 'GitHub', url: 'https://github.com/alice' },
    { title: 'Website', url: 'https://alice.dev' }
  ]
)
```

---

## Integration with Rails

### Service Object

```ruby
# app/services/pubky_post_service.rb
class PubkyPostService
  def self.create(content:, kind: 'short', parent: nil)
    if parent
      PubkyFFI.create_post_full(
        content: content,
        kind: kind,
        parent: parent
      )
    else
      PubkyFFI.create_post(content: content, kind: kind)
    end
  end

  def self.validate(post_data, id: nil)
    PubkyFFI.validate_post(post_data, id: id)
  end
end
```

### Controller

```ruby
# app/controllers/posts_controller.rb
class PostsController < ApplicationController
  def create
    result = PubkyPostService.create(
      content: params[:content],
      kind: params[:kind] || 'short',
      parent: params[:parent_uri]
    )

    if result[:success]
      render json: {
        id: result[:id],
        path: result[:path],
        post: result[:post]
      }, status: :created
    else
      render json: { error: result[:error] }, status: :unprocessable_entity
    end
  end
end
```

### Background Job

```ruby
# app/jobs/create_pubky_post_job.rb
class CreatePubkyPostJob < ApplicationJob
  queue_as :default

  def perform(content, kind = 'short')
    result = PubkyFFI.create_post(content: content, kind: kind)

    if result[:success]
      Rails.logger.info "Created post #{result[:id]}"
    else
      Rails.logger.error "Failed to create post: #{result[:error]}"
      raise result[:error]
    end
  end
end
```

### Custom Validator

```ruby
# app/validators/pubky_post_validator.rb
class PubkyPostValidator < ActiveModel::Validator
  def validate(record)
    result = PubkyFFI.validate_post({
      content: record.content,
      kind: record.kind || 'short'
    })

    unless result[:valid]
      record.errors.add(:base, result[:error])
    end
  end
end

# app/models/post.rb
class Post < ApplicationRecord
  validates_with PubkyPostValidator
end
```

---

## Why Not WASM?

The `pubky-app-specs` WASM module is built using `wasm-bindgen` specifically for **JavaScript environments**. It includes JS glue code that handles memory allocation, string conversions, and type bindings.

**This WASM module cannot be directly used with Ruby WASM runtimes** (wasmtime/wasmer) because:

1. The WASM file expects JavaScript glue code to be present
2. Reimplementing the glue layer in Ruby would require hundreds of lines of complex code
3. Memory management, type conversions, and ABI compatibility are non-trivial

**FFI is the recommended approach** because:
- It uses the standard C ABI that Ruby's `ffi` gem handles natively
- Memory management is explicit and well-understood
- Native performance without any overhead
- The FFI exports are now included in the library (`src/ffi.rs`)

---

## Memory Management

All functions that return strings allocate memory on the Rust side. The Ruby wrapper automatically frees this memory using `pubky_free_string`. If you're calling the FFI functions directly without the wrapper, remember to free returned pointers:

```ruby
# Direct FFI usage (not recommended)
ptr = PubkyFFI.pubky_create_post("Hello", "short")
result = ptr.read_string
PubkyFFI.pubky_free_string(ptr)  # Don't forget this!
```

---

## Thread Safety

All FFI functions are thread-safe and can be called from multiple threads concurrently. The Ruby wrapper methods are also thread-safe.

---

## License

This integration guide follows the MIT License of the pubky-app-specs project.
