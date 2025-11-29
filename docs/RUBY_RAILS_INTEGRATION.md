# Ruby on Rails Integration Guide

This guide explains how to use the Pubky.app data model specifications (`pubky-app-specs`) in a Ruby on Rails application. There are two approaches:

1. **WebAssembly (WASM)** - Currently available, using the `wasmtime` gem
2. **FFI (Foreign Function Interface)** - Possible with additional Rust code modifications

## Table of Contents

- [Option 1: Using the WASM Module](#option-1-using-the-wasm-module)
  - [Installation](#installation)
  - [Loading the WASM Module](#loading-the-wasm-module)
  - [Creating and Validating Objects](#creating-and-validating-objects)
  - [Integration with Rails](#integration-with-rails)
- [Option 2: Using FFI](#option-2-using-ffi)
  - [Current State](#current-state)
  - [How FFI Would Work](#how-ffi-would-work)
  - [Required Rust Changes](#required-rust-changes)
  - [Ruby FFI Usage](#ruby-ffi-usage)

---

## Option 1: Using the WASM Module

### Installation

> **Note**: The `wasmtime` gem is a **server-side** WebAssembly runtime. It runs WASM code directly on your server, not in a browser. This means you can use the WASM approach in Rails controllers, background jobs (Sidekiq, ActiveJob, etc.), rake tasks, and any other server-side Ruby code.

Add to your Gemfile:

```ruby
gem 'wasmtime', '~> 29.0'
```

Then run:

```bash
bundle install
```

Download the WASM module from the npm package:

```bash
# From npm (recommended for version tracking)
npm pack pubky-app-specs@0.4.0
tar -xzf pubky-app-specs-0.4.0.tgz
mkdir -p lib/wasm
cp package/pubky_app_specs_bg.wasm lib/wasm/pubky_app_specs_bg-0.4.0.wasm
```

### Versioning Strategy

We recommend including the version number in the WASM filename (e.g., `pubky_app_specs_bg-0.4.0.wasm`) for several reasons:

1. **Clear version tracking**: Easily identify which version is deployed
2. **Safe upgrades**: Keep old versions alongside new ones during migration
3. **Cache busting**: Avoid stale module issues when upgrading
4. **Rollback capability**: Quickly switch back to a previous version if needed

To upgrade to a new version:
1. Download the new npm package: `npm pack pubky-app-specs@NEW_VERSION`
2. Extract and copy the WASM file with versioned name
3. Update the `VERSION` constant in your initializer
4. Test thoroughly before removing the old WASM file

---

## Loading the WASM Module

Create an initializer to load and configure the WASM module:

```ruby
# config/initializers/pubky_specs.rb
require 'wasmtime'

module PubkySpecs
  VERSION = '0.4.0'
  WASM_FILENAME = "pubky_app_specs_bg-#{VERSION}.wasm"

  class << self
    def engine
      @engine ||= Wasmtime::Engine.new
    end

    def wasm_module
      @wasm_module ||= begin
        wasm_path = Rails.root.join('lib', 'wasm', WASM_FILENAME)
        Wasmtime::Module.from_file(engine, wasm_path.to_s)
      end
    end

    def store
      @store ||= Wasmtime::Store.new(engine)
    end

    def linker
      @linker ||= Wasmtime::Linker.new(engine, wasi: true)
    end

    def instance
      @instance ||= linker.instantiate(store, wasm_module)
    end

    # Helper to call exported WASM functions
    def call_function(name, *args)
      func = instance.export(name).to_func
      func.call(*args)
    end
  end
end
```

---

## Creating and Validating Objects

The `pubky-app-specs` WASM module exports a `PubkySpecsBuilder` class that provides methods to create and validate Pubky objects. Each method automatically:
- Sanitizes the input data
- Validates the object against the spec
- Generates a unique ID
- Returns both the object and its metadata (id, path, url)

### Creating a PubkyAppPost

Here's how to create and validate a post:

```ruby
# app/services/pubky_post_service.rb
require 'json'

class PubkyPostService
  # User's Pubky ID (public key)
  PUBKY_ID = 'your_pubky_id_here'

  def self.create_post(content:, kind: 'short', parent: nil, embed: nil, attachments: nil)
    # The WASM module's createPost function validates and creates the post
    # It returns a PostResult with { post, meta } where:
    # - post: the sanitized PubkyAppPost object
    # - meta: { id, path, url } for the created post
    
    result = PubkySpecs.call_function(
      'createPost',
      PUBKY_ID,
      content,
      kind,
      parent,
      embed,
      attachments
    )

    # Parse the result
    {
      success: true,
      post: result['post'],
      meta: {
        id: result['meta']['id'],
        path: result['meta']['path'],
        url: result['meta']['url']
      }
    }
  rescue => e
    # Validation errors are raised as exceptions
    { success: false, error: e.message }
  end

  def self.validate_post(content:, kind: 'short')
    result = create_post(content: content, kind: kind)
    result[:success]
  end
end

# Usage examples:

# Create a simple short post
result = PubkyPostService.create_post(
  content: 'Hello, Pubky world! This is my first post.',
  kind: 'short'
)

if result[:success]
  puts "Post created successfully!"
  puts "Post ID: #{result[:meta][:id]}"
  puts "Post URL: #{result[:meta][:url]}"
  puts "Post path: #{result[:meta][:path]}"
else
  puts "Error: #{result[:error]}"
end

# Create a reply to another post
parent_uri = 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0'
reply = PubkyPostService.create_post(
  content: 'This is a reply!',
  kind: 'short',
  parent: parent_uri
)

# Validate post content without creating
is_valid = PubkyPostService.validate_post(
  content: 'Test content',
  kind: 'short'
)
puts "Post is valid: #{is_valid}"
```

### Creating a PubkyAppFile

```ruby
# app/services/pubky_file_service.rb
class PubkyFileService
  PUBKY_ID = 'your_pubky_id_here'

  def self.create_file(name:, src:, content_type:, size:)
    result = PubkySpecs.call_function(
      'createFile',
      PUBKY_ID,
      name,
      src,
      content_type,
      size
    )

    {
      success: true,
      file: result['file'],
      meta: {
        id: result['meta']['id'],
        path: result['meta']['path'],
        url: result['meta']['url']
      }
    }
  rescue => e
    { success: false, error: e.message }
  end
end

# Usage:
result = PubkyFileService.create_file(
  name: 'photo.jpg',
  src: 'pubky://user123/pub/pubky.app/blobs/ABC123',
  content_type: 'image/jpeg',
  size: 512_000  # 500KB
)

if result[:success]
  puts "File ID: #{result[:meta][:id]}"
  puts "File URL: #{result[:meta][:url]}"
end
```

### Creating Other Objects

The WASM module provides similar methods for all Pubky object types:

```ruby
# Create a bookmark
PubkySpecs.call_function('createBookmark', pubky_id, uri)

# Create a tag
PubkySpecs.call_function('createTag', pubky_id, uri, label)

# Create a follow
PubkySpecs.call_function('createFollow', pubky_id, followee_id)

# Create a mute
PubkySpecs.call_function('createMute', pubky_id, mutee_id)

# Create a user profile
PubkySpecs.call_function('createUser', pubky_id, name, bio, image, links, status)

# Create a feed
PubkySpecs.call_function('createFeed', pubky_id, tags, reach, layout, sort, content, name)

# Create a blob
PubkySpecs.call_function('createBlob', pubky_id, blob_data)
```

---

## Integration with Rails

### Using in Controllers

```ruby
# app/controllers/posts_controller.rb
class PostsController < ApplicationController
  def create
    result = PubkyPostService.create_post(
      content: params[:content],
      kind: params[:kind] || 'short',
      parent: params[:parent_uri]
    )

    if result[:success]
      render json: {
        id: result[:meta][:id],
        url: result[:meta][:url],
        post: result[:post]
      }, status: :created
    else
      render json: { error: result[:error] }, status: :unprocessable_entity
    end
  end
end
```

### Using in Background Jobs

```ruby
# app/jobs/create_pubky_post_job.rb
class CreatePubkyPostJob < ApplicationJob
  queue_as :default

  def perform(user_id, content, kind = 'short')
    result = PubkyPostService.create_post(
      content: content,
      kind: kind
    )

    if result[:success]
      # Store the result, notify user, etc.
      Rails.logger.info "Created post #{result[:meta][:id]} for user #{user_id}"
    else
      Rails.logger.error "Failed to create post: #{result[:error]}"
    end
  end
end
```

### Custom Validator

```ruby
# app/validators/pubky_post_validator.rb
class PubkyPostValidator < ActiveModel::Validator
  def validate(record)
    result = PubkyPostService.create_post(
      content: record.content,
      kind: record.kind || 'short',
      parent: record.parent_uri
    )

    unless result[:success]
      record.errors.add(:base, result[:error])
    end
  end
end

# app/models/post.rb
class Post < ApplicationRecord
  validates_with PubkyPostValidator
end
```

### Service Object Pattern

```ruby
# app/services/pubky_content_creator.rb
class PubkyContentCreator
  def initialize(pubky_id)
    @pubky_id = pubky_id
  end

  def create_post(content:, kind: 'short', parent: nil)
    call_wasm('createPost', content, kind, parent, nil, nil)
  end

  def create_file(name:, src:, content_type:, size:)
    call_wasm('createFile', name, src, content_type, size)
  end

  def create_bookmark(uri:)
    call_wasm('createBookmark', uri)
  end

  def create_tag(uri:, label:)
    call_wasm('createTag', uri, label)
  end

  private

  def call_wasm(method, *args)
    result = PubkySpecs.call_function(method, @pubky_id, *args)
    {
      success: true,
      data: result['data'] || result[method.sub('create', '').downcase],
      meta: result['meta']
    }
  rescue => e
    { success: false, error: e.message }
  end
end

# Usage:
creator = PubkyContentCreator.new('user_pubky_id')

post = creator.create_post(content: 'Hello World!')
file = creator.create_file(
  name: 'doc.pdf',
  src: 'pubky://user/pub/pubky.app/blobs/123',
  content_type: 'application/pdf',
  size: 1024
)
```

---

## Notes (WASM)

1. **Thread Safety**: Create a new `Wasmtime::Store` for each thread if using in a multi-threaded environment.

2. **Error Handling**: The WASM module raises descriptive errors when validation fails. Always wrap calls in error handling.

3. **Performance**: The WASM module is loaded once at startup. Subsequent calls are fast.

4. **Validation Rules**: The validation rules in the WASM module match the Rust implementation exactly. See the main README for detailed specifications.

---

## Option 2: Using FFI

FFI (Foreign Function Interface) allows Ruby to directly call functions in native shared libraries compiled from Rust. This can provide better performance than WASM for certain use cases.

### Current State

The `pubky-app-specs` Rust library is configured with `crate-type = ["cdylib", "rlib"]` in `Cargo.toml`, which means it **can** be compiled to a native shared library (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows).

However, the library currently doesn't export C-compatible FFI functions (no `#[no_mangle] extern "C"` functions). To use FFI, additional wrapper functions need to be added to the Rust codebase.

### How FFI Would Work

Once FFI exports are added to the Rust library, the integration would work as follows:

1. **Compile the Rust library** to a native shared library
2. **Use the `ffi` gem** in Ruby to load and call the functions
3. **Handle memory** carefully (strings, structs, etc.)

### Required Rust Changes

To enable FFI, the following changes would need to be made to the Rust library. Here's an example of what FFI exports might look like:

```rust
// src/ffi.rs - Example FFI wrapper functions

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use crate::{PubkyAppPost, PubkyAppPostKind};
use crate::traits::{TimestampId, Validatable};

/// Free a string allocated by Rust
/// 
/// # Safety
/// The pointer must be a valid CString allocated by Rust
#[no_mangle]
pub unsafe extern "C" fn pubky_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Create and validate a post, returning JSON result
/// 
/// # Safety
/// The content pointer must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn pubky_create_post(
    content: *const c_char,
    kind: *const c_char,
) -> *mut c_char {
    // Convert C strings to Rust strings
    let content = match CStr::from_ptr(content).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    
    let kind_str = match CStr::from_ptr(kind).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let kind = match kind_str {
        "short" => PubkyAppPostKind::Short,
        "long" => PubkyAppPostKind::Long,
        "image" => PubkyAppPostKind::Image,
        "video" => PubkyAppPostKind::Video,
        "link" => PubkyAppPostKind::Link,
        "file" => PubkyAppPostKind::File,
        _ => PubkyAppPostKind::Short,
    };
    
    // Create and sanitize the post
    let post = PubkyAppPost::new(content, kind, None, None, None);
    let post_id = post.create_id();
    
    // Validate
    if let Err(e) = post.validate(Some(&post_id)) {
        let error_json = format!(r#"{{"error": "{}"}}"#, e);
        return CString::new(error_json).unwrap().into_raw();
    }
    
    // Return JSON result
    let result = serde_json::json!({
        "success": true,
        "id": post_id,
        "post": {
            "content": post.content,
            "kind": kind_str,
        }
    });
    
    CString::new(result.to_string()).unwrap().into_raw()
}

/// Validate a post without creating it
/// 
/// # Safety
/// The json_input pointer must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_post(json_input: *const c_char) -> *mut c_char {
    let json_str = match CStr::from_ptr(json_input).to_str() {
        Ok(s) => s,
        Err(_) => {
            return CString::new(r#"{"valid": false, "error": "Invalid UTF-8"}"#)
                .unwrap()
                .into_raw();
        }
    };
    
    // Parse JSON and validate
    match serde_json::from_str::<PubkyAppPost>(json_str) {
        Ok(post) => {
            let post = post.sanitize();
            match post.validate(None) {
                Ok(_) => CString::new(r#"{"valid": true}"#).unwrap().into_raw(),
                Err(e) => {
                    let result = format!(r#"{{"valid": false, "error": "{}"}}"#, e);
                    CString::new(result).unwrap().into_raw()
                }
            }
        }
        Err(e) => {
            let result = format!(r#"{{"valid": false, "error": "{}"}}"#, e);
            CString::new(result).unwrap().into_raw()
        }
    }
}

/// Generate a timestamp-based ID for posts/files
#[no_mangle]
pub extern "C" fn pubky_generate_timestamp_id() -> *mut c_char {
    let post = PubkyAppPost::new(
        String::new(),
        PubkyAppPostKind::Short,
        None,
        None,
        None,
    );
    let id = post.create_id();
    CString::new(id).unwrap().into_raw()
}
```

The `Cargo.toml` already has the correct crate-type, but would need the FFI module added:

```rust
// In src/lib.rs, add:
#[cfg(not(target_arch = "wasm32"))]
mod ffi;
```

### Building the Shared Library

Once FFI exports are added, compile the library:

```bash
# Linux
cargo build --release
# Output: target/release/libpubky_app_specs.so

# macOS
cargo build --release
# Output: target/release/libpubky_app_specs.dylib

# Windows
cargo build --release
# Output: target/release/pubky_app_specs.dll
```

### Ruby FFI Usage

Once the shared library is compiled with FFI exports, you can use it from Ruby:

```ruby
# Gemfile
gem 'ffi', '~> 1.16'
```

```ruby
# config/initializers/pubky_ffi.rb
require 'ffi'

module PubkyFFI
  extend FFI::Library
  
  # Load the shared library based on platform
  lib_name = case RbConfig::CONFIG['host_os']
             when /darwin/
               'libpubky_app_specs.dylib'
             when /linux/
               'libpubky_app_specs.so'
             when /mswin|mingw/
               'pubky_app_specs.dll'
             end
  
  ffi_lib Rails.root.join('lib', 'native', lib_name).to_s
  
  # Define the FFI function signatures
  attach_function :pubky_create_post, [:string, :string], :pointer
  attach_function :pubky_validate_post, [:string], :pointer
  attach_function :pubky_generate_timestamp_id, [], :pointer
  attach_function :pubky_free_string, [:pointer], :void
  
  class << self
    def create_post(content:, kind: 'short')
      result_ptr = pubky_create_post(content, kind)
      result = result_ptr.read_string
      pubky_free_string(result_ptr)
      JSON.parse(result, symbolize_names: true)
    end
    
    def validate_post(json_data)
      json_str = json_data.is_a?(String) ? json_data : json_data.to_json
      result_ptr = pubky_validate_post(json_str)
      result = result_ptr.read_string
      pubky_free_string(result_ptr)
      JSON.parse(result, symbolize_names: true)
    end
    
    def generate_id
      result_ptr = pubky_generate_timestamp_id
      result = result_ptr.read_string
      pubky_free_string(result_ptr)
      result
    end
  end
end
```

```ruby
# Usage example
result = PubkyFFI.create_post(
  content: 'Hello from FFI!',
  kind: 'short'
)

if result[:success]
  puts "Post ID: #{result[:id]}"
else
  puts "Error: #{result[:error]}"
end

# Validate a post
validation = PubkyFFI.validate_post({
  content: 'Test content',
  kind: 'short'
})
puts "Valid: #{validation[:valid]}"

# Generate an ID
new_id = PubkyFFI.generate_id
puts "Generated ID: #{new_id}"
```

### FFI vs WASM Comparison

| Feature | WASM (wasmtime) | FFI |
|---------|-----------------|-----|
| **Availability** | Ready to use now | Requires Rust code changes |
| **Performance** | Good | Potentially faster |
| **Setup complexity** | Simple | Moderate |
| **Platform support** | Excellent (WASM is portable) | Requires per-platform compilation |
| **Memory safety** | Sandboxed | Requires careful handling |
| **Deployment** | Single WASM file | Platform-specific binaries |

### When to Choose FFI

Consider FFI over WASM when:
- You need maximum performance for high-throughput validation
- You're already using other native libraries via FFI
- You have a controlled deployment environment where you can compile for specific platforms
- The WASM overhead is measurable and significant for your use case

For most Rails applications, the **WASM approach is recommended** due to its simplicity and portability.

---

## License

This integration guide follows the MIT License of the pubky-app-specs project.
