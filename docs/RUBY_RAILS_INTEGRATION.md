# Ruby on Rails Integration Guide

This guide explains how to use the Pubky.app data model specifications (`pubky-app-specs`) in a Ruby on Rails application using the WebAssembly (WASM) module via the `wasmtime` gem.

## Table of Contents

- [Installation](#installation)
- [Loading the WASM Module](#loading-the-wasm-module)
- [Creating and Validating Objects](#creating-and-validating-objects)
  - [Creating a PubkyAppPost](#creating-a-pubkyapppost)
  - [Creating a PubkyAppFile](#creating-a-pubkyappfile)
  - [Creating Other Objects](#creating-other-objects)
- [Integration with Rails](#integration-with-rails)

---

## Installation

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

## Notes

1. **Thread Safety**: Create a new `Wasmtime::Store` for each thread if using in a multi-threaded environment.

2. **Error Handling**: The WASM module raises descriptive errors when validation fails. Always wrap calls in error handling.

3. **Performance**: The WASM module is loaded once at startup. Subsequent calls are fast.

4. **Validation Rules**: The validation rules in the WASM module match the Rust implementation exactly. See the main README for detailed specifications.

---

## License

This integration guide follows the MIT License of the pubky-app-specs project.
