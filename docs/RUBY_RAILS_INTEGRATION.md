# Ruby on Rails Integration Guide

This guide explains how to use the Pubky.app data model specifications (`pubky-app-specs`) in a Ruby on Rails application.

## Important: WASM Module Compatibility

The `pubky-app-specs` WASM module is built using `wasm-bindgen` specifically for **JavaScript environments**. It includes JS glue code that handles:
- Memory allocation and deallocation
- String conversions between JavaScript and WASM
- Complex object serialization/deserialization
- Function binding and type conversions

**This means the WASM module cannot be directly used with Ruby WASM runtimes** like `wasmtime` or `wasmer` without the JavaScript glue layer. Attempting to load and call the WASM functions directly would require manually reimplementing all the `wasm-bindgen` glue code, which is extremely complex and error-prone.

## Available Integration Options

For Ruby on Rails integration, we recommend using **FFI (Foreign Function Interface)** which requires additional Rust code changes to the library:

1. **FFI (Foreign Function Interface)** - The practical approach for Ruby/Rails
   - Requires adding C-compatible exports to the Rust library
   - Provides direct native performance
   - Works reliably across platforms

## Table of Contents

- [Option 1: Using FFI (Recommended)](#option-1-using-ffi-recommended)
  - [Current State](#current-state)
  - [Required Rust Changes](#required-rust-changes)
  - [Building the Shared Library](#building-the-shared-library)
  - [Ruby FFI Usage](#ruby-ffi-usage)
  - [Integration with Rails](#integration-with-rails)
- [Why Not WASM?](#why-not-wasm)

---

## Option 1: Using FFI (Recommended)

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

### Integration with Rails

Once FFI exports are added, you can use the same Rails integration patterns shown below:

#### Using in Controllers

```ruby
# app/controllers/posts_controller.rb
class PostsController < ApplicationController
  def create
    result = PubkyFFI.create_post(
      content: params[:content],
      kind: params[:kind] || 'short'
    )

    if result[:success]
      render json: { id: result[:id], post: result[:post] }, status: :created
    else
      render json: { error: result[:error] }, status: :unprocessable_entity
    end
  end
end
```

#### Using in Background Jobs

```ruby
# app/jobs/create_pubky_post_job.rb
class CreatePubkyPostJob < ApplicationJob
  queue_as :default

  def perform(user_id, content, kind = 'short')
    result = PubkyFFI.create_post(content: content, kind: kind)

    if result[:success]
      Rails.logger.info "Created post #{result[:id]} for user #{user_id}"
    else
      Rails.logger.error "Failed to create post: #{result[:error]}"
    end
  end
end
```

#### Custom Validator

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

The `pubky-app-specs` WASM module is built using `wasm-bindgen` for JavaScript environments. Here's why it cannot be directly used with Ruby WASM runtimes:

### The Problem

When `wasm-bindgen` builds a WASM module, it generates:

1. **A `.wasm` file** - The compiled WebAssembly binary
2. **JavaScript glue code** - Essential code that handles:
   - Memory allocation for strings and complex objects
   - Type conversions between JS and WASM
   - Function bindings with proper signatures
   - Cleanup and garbage collection coordination

The WASM file alone is **not self-contained** - it expects the JavaScript glue code to be present and to handle all the complex interop.

### What Would Be Required

To use the WASM module directly from Ruby (via `wasmtime` or `wasmer`), you would need to:

1. **Reimplement the entire `wasm-bindgen` glue layer in Ruby** - This is hundreds of lines of complex code
2. **Handle WASM linear memory manually** - Allocate/deallocate strings, manage memory layout
3. **Implement the same type conversions** - Ruby types ↔ WASM memory representation
4. **Match the exact ABI** that `wasm-bindgen` uses - Function signatures, memory layout, etc.

This is extremely complex and error-prone, making it impractical for production use.

### The Solution

**FFI is the recommended approach** for Ruby/Rails integration because:

- It uses standard C ABI which Ruby's `ffi` gem handles natively
- Memory management is explicit and well-understood
- No JavaScript dependency or glue code required
- Native performance without WASM overhead

The trade-off is that FFI exports need to be added to the Rust library, but once added, the integration is clean and maintainable.

---

## License

This integration guide follows the MIT License of the pubky-app-specs project.
