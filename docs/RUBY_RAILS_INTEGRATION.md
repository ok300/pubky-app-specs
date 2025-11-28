# Ruby on Rails Integration Guide

This guide explains how to use the Pubky.app data model specifications (`pubky-app-specs`) in a Ruby on Rails application. You can either use the WebAssembly (WASM) module directly via the `wasmer` or `wasmtime` gems, or implement the validation and ID generation logic in pure Ruby.

## Table of Contents

- [Option 1: Using the WASM Module](#option-1-using-the-wasm-module)
  - [Installation](#installation)
  - [Loading the WASM Module](#loading-the-wasm-module)
  - [Creating and Validating Objects](#creating-and-validating-objects)
- [Option 2: Pure Ruby Implementation](#option-2-pure-ruby-implementation)
  - [Dependencies](#dependencies)
  - [Core Modules](#core-modules)
  - [Model Examples](#model-examples)
- [Validation Examples](#validation-examples)
- [ID Generation Examples](#id-generation-examples)
- [Integration with Rails Models](#integration-with-rails-models)

---

## Option 1: Using the WASM Module

The `pubky-app-specs` library is compiled to WebAssembly and can be used in Ruby via the `wasmer` gem.

### Installation

Add to your Gemfile:

```ruby
gem 'wasmer', '~> 1.2'
```

Then run:

```bash
bundle install
```

Download the WASM module from npm or build it from source:

```bash
# From npm
npm pack pubky-app-specs
tar -xzf pubky-app-specs-*.tgz
cp package/pubky_app_specs_bg.wasm lib/wasm/

# Or build from source
cd pubky-app-specs
wasm-pack build --target bundler
cp pkg/pubky_app_specs_bg.wasm /path/to/your/rails/app/lib/wasm/
```

### Loading the WASM Module

```ruby
# config/initializers/pubky_specs.rb
require 'wasmer'

module PubkySpecs
  class << self
    def store
      @store ||= Wasmer::Store.new
    end

    def wasm_module
      @wasm_module ||= begin
        wasm_path = Rails.root.join('lib', 'wasm', 'pubky_app_specs_bg.wasm')
        Wasmer::Module.new(store, File.read(wasm_path, mode: 'rb'))
      end
    end

    def instance
      @instance ||= Wasmer::Instance.new(wasm_module, nil)
    end
  end
end
```

---

## Option 2: Pure Ruby Implementation

For applications that prefer native Ruby without WASM dependencies, here's how to implement the core functionality.

### Dependencies

Add to your Gemfile:

```ruby
gem 'base32', '~> 0.3'  # For Crockford Base32 encoding
gem 'blake3', '~> 1.5'  # For Blake3 hashing
```

Then run:

```bash
bundle install
```

### Core Modules

Create the following modules in your Rails application:

#### lib/pubky_app_specs/base.rb

```ruby
# frozen_string_literal: true

module PubkyAppSpecs
  VERSION = '0.4.0'
  PROTOCOL = 'pubky://'
  PUBLIC_PATH = '/pub/'
  APP_PATH = 'pubky.app/'
  MAX_SIZE = 100 * (1 << 20) # 100MB

  # Crockford Base32 alphabet
  CROCKFORD_ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'

  module_function

  # Returns current timestamp in microseconds
  def timestamp
    (Time.now.to_f * 1_000_000).to_i
  end

  # Encode bytes to Crockford Base32
  def crockford_encode(bytes)
    # Convert bytes to binary string for bit manipulation
    bits = bytes.map { |b| b.to_s(2).rjust(8, '0') }.join

    # Pad to multiple of 5
    bits += '0' * ((5 - bits.length % 5) % 5)

    # Convert each 5-bit group to character
    result = ''
    bits.scan(/.{5}/).each do |group|
      result += CROCKFORD_ALPHABET[group.to_i(2)]
    end

    result
  end

  # Decode Crockford Base32 to bytes
  def crockford_decode(str)
    # Normalize input (uppercase, handle common substitutions)
    normalized = str.upcase.tr('OIL', '010')

    bits = ''
    normalized.each_char do |c|
      idx = CROCKFORD_ALPHABET.index(c)
      return nil if idx.nil?

      bits += idx.to_s(2).rjust(5, '0')
    end

    # Convert to bytes
    bytes = []
    bits.scan(/.{8}/).each do |byte_str|
      bytes << byte_str.to_i(2)
    end

    bytes.pack('C*')
  end
end
```

#### lib/pubky_app_specs/id_generators.rb

```ruby
# frozen_string_literal: true

require 'blake3'

module PubkyAppSpecs
  module IdGenerators
    module_function

    # Creates a timestamp-based ID (for Posts, Files, etc.)
    # Returns a 13-character Crockford Base32 string
    def create_timestamp_id
      timestamp_micros = PubkyAppSpecs.timestamp
      bytes = [timestamp_micros].pack('Q>').bytes
      PubkyAppSpecs.crockford_encode(bytes)
    end

    # Creates a hash-based ID (for Tags, Bookmarks, etc.)
    # The ID is derived from Blake3 hash of the input data
    def create_hash_id(data)
      hasher = Blake3::Hasher.new
      hasher.update(data)
      hash_bytes = hasher.digest

      # Take first half of the hash (16 bytes)
      half_hash = hash_bytes.bytes[0, 16]
      PubkyAppSpecs.crockford_encode(half_hash)
    end

    # Validates a timestamp ID
    def validate_timestamp_id(id)
      return { valid: false, error: 'ID must be 13 characters' } unless id.length == 13

      decoded = PubkyAppSpecs.crockford_decode(id)
      return { valid: false, error: 'Invalid Crockford Base32 encoding' } if decoded.nil?

      timestamp_micros = decoded.unpack1('Q>')
      now_micros = PubkyAppSpecs.timestamp

      # October 1st, 2024 in microseconds
      oct_first_2024 = 1_727_740_800_000_000

      if timestamp_micros < oct_first_2024
        return { valid: false, error: 'Timestamp must be after October 1st, 2024' }
      end

      # Allow up to 2 hours in the future
      max_future = now_micros + (2 * 60 * 60 * 1_000_000)
      if timestamp_micros > max_future
        return { valid: false, error: 'Timestamp is too far in the future' }
      end

      { valid: true }
    end
  end
end
```

#### lib/pubky_app_specs/validators.rb

```ruby
# frozen_string_literal: true

require 'uri'
require 'json'

module PubkyAppSpecs
  module Validators
    VALID_MIME_TYPES = %w[
      application/javascript application/json application/octet-stream
      application/pdf application/x-www-form-urlencoded application/xml
      application/zip audio/mpeg audio/wav image/gif image/jpeg image/png
      image/svg+xml image/webp multipart/form-data text/css text/html
      text/plain text/xml video/mp4 video/mpeg
    ].freeze

    POST_KINDS = %w[short long image video link file].freeze
    FEED_REACHES = %w[all following followers friends].freeze
    FEED_LAYOUTS = %w[columns grid list].freeze
    FEED_SORTS = %w[recent popular].freeze

    module_function

    def valid_url?(str)
      uri = URI.parse(str)
      uri.is_a?(URI::HTTP) || uri.is_a?(URI::HTTPS) || str.start_with?('pubky://')
    rescue URI::InvalidURIError
      false
    end

    def valid_pubky_uri?(str)
      str.start_with?('pubky://') && str.include?('/pub/pubky.app/')
    end
  end
end
```

### Model Examples

#### lib/pubky_app_specs/models/post.rb

```ruby
# frozen_string_literal: true

module PubkyAppSpecs
  module Models
    class Post
      include PubkyAppSpecs::IdGenerators

      MAX_SHORT_CONTENT_LENGTH = 2000
      MAX_LONG_CONTENT_LENGTH = 50_000
      PATH_SEGMENT = 'posts/'
      VALID_KINDS = %w[short long image video link file].freeze

      attr_accessor :content, :kind, :parent, :embed, :attachments

      def initialize(content:, kind: 'short', parent: nil, embed: nil, attachments: nil)
        @content = content
        @kind = kind
        @parent = parent
        @embed = embed
        @attachments = attachments
        sanitize!
      end

      # Create a timestamp-based ID for this post
      def create_id
        IdGenerators.create_timestamp_id
      end

      # Generate the storage path for this post
      def self.create_path(id)
        "#{PUBLIC_PATH}#{APP_PATH}#{PATH_SEGMENT}#{id}"
      end

      # Generate full URI
      def self.create_uri(user_id, post_id)
        "#{PROTOCOL}#{user_id}#{create_path(post_id)}"
      end

      # Sanitize the post content
      def sanitize!
        @content = @content.to_s.strip

        # Reserved keyword for deleted posts
        @content = 'empty' if @content == '[DELETED]'

        # Truncate based on kind
        max_length = @kind == 'long' ? MAX_LONG_CONTENT_LENGTH : MAX_SHORT_CONTENT_LENGTH
        @content = @content[0, max_length] if @content.length > max_length

        # Validate parent URI
        @parent = nil unless @parent.nil? || Validators.valid_url?(@parent)

        # Validate embed
        if @embed && !Validators.valid_url?(@embed[:uri].to_s)
          @embed = nil
        end

        self
      end

      # Validate the post
      def validate(id: nil)
        errors = []

        # Validate ID if provided
        if id
          result = IdGenerators.validate_timestamp_id(id)
          errors << result[:error] unless result[:valid]
        end

        # Validate kind
        unless VALID_KINDS.include?(@kind)
          errors << "Invalid post kind: #{@kind}"
        end

        # Validate content length
        max_length = @kind == 'long' ? MAX_LONG_CONTENT_LENGTH : MAX_SHORT_CONTENT_LENGTH
        if @content.length > max_length
          errors << "Content exceeds maximum length for #{@kind} post"
        end

        # Validate content is not the reserved keyword
        if @content == '[DELETED]'
          errors << 'Content cannot be [DELETED]'
        end

        # Validate parent URI if present
        if @parent && !Validators.valid_url?(@parent)
          errors << 'Invalid parent URI'
        end

        # Validate embed if present
        if @embed
          unless @embed[:uri] && Validators.valid_url?(@embed[:uri])
            errors << 'Invalid embed URI'
          end
          unless @embed[:kind] && VALID_KINDS.include?(@embed[:kind])
            errors << 'Invalid embed kind'
          end
        end

        # Validate attachments if present
        @attachments&.each_with_index do |uri, idx|
          unless Validators.valid_url?(uri)
            errors << "Invalid attachment URI at index #{idx}"
          end
        end

        errors.empty? ? { valid: true } : { valid: false, errors: errors }
      end

      # Serialize to JSON
      def to_json(*_args)
        {
          content: @content,
          kind: @kind,
          parent: @parent,
          embed: @embed,
          attachments: @attachments
        }.compact.to_json
      end

      # Parse from JSON
      def self.from_json(json_str, id: nil)
        data = JSON.parse(json_str, symbolize_names: true)
        post = new(
          content: data[:content],
          kind: data[:kind] || 'short',
          parent: data[:parent],
          embed: data[:embed],
          attachments: data[:attachments]
        )

        result = post.validate(id: id)
        raise "Validation failed: #{result[:errors].join(', ')}" unless result[:valid]

        post
      end
    end
  end
end
```

#### lib/pubky_app_specs/models/file.rb

```ruby
# frozen_string_literal: true

module PubkyAppSpecs
  module Models
    class File
      include PubkyAppSpecs::IdGenerators

      MIN_NAME_LENGTH = 1
      MAX_NAME_LENGTH = 255
      MAX_SRC_LENGTH = 1024
      PATH_SEGMENT = 'files/'

      attr_accessor :name, :created_at, :src, :content_type, :size

      def initialize(name:, src:, content_type:, size:, created_at: nil)
        @name = name
        @src = src
        @content_type = content_type
        @size = size
        @created_at = created_at || PubkyAppSpecs.timestamp
        sanitize!
      end

      # Create a timestamp-based ID for this file
      def create_id
        IdGenerators.create_timestamp_id
      end

      # Generate the storage path for this file
      def self.create_path(id)
        "#{PUBLIC_PATH}#{APP_PATH}#{PATH_SEGMENT}#{id}"
      end

      # Generate full URI
      def self.create_uri(user_id, file_id)
        "#{PROTOCOL}#{user_id}#{create_path(file_id)}"
      end

      # Sanitize the file metadata
      def sanitize!
        @name = @name.to_s.strip[0, MAX_NAME_LENGTH]
        @src = @src.to_s.strip[0, MAX_SRC_LENGTH]
        @content_type = @content_type.to_s.strip

        # Validate src is a URL
        @src = '' unless Validators.valid_url?(@src)

        self
      end

      # Validate the file
      def validate(id: nil)
        errors = []

        # Validate ID if provided
        if id
          result = IdGenerators.validate_timestamp_id(id)
          errors << result[:error] unless result[:valid]
        end

        # Validate name
        if @name.length < MIN_NAME_LENGTH || @name.length > MAX_NAME_LENGTH
          errors << "Name must be between #{MIN_NAME_LENGTH} and #{MAX_NAME_LENGTH} characters"
        end

        # Validate src
        if @src.empty?
          errors << 'Source URL is required'
        elsif @src.length > MAX_SRC_LENGTH
          errors << "Source URL exceeds maximum length of #{MAX_SRC_LENGTH}"
        elsif !Validators.valid_url?(@src)
          errors << 'Invalid source URL'
        end

        # Validate content type
        unless Validators::VALID_MIME_TYPES.include?(@content_type)
          errors << "Invalid content type: #{@content_type}"
        end

        # Validate size
        if @size <= 0 || @size > PubkyAppSpecs::MAX_SIZE
          errors << "Size must be between 1 and #{PubkyAppSpecs::MAX_SIZE} bytes"
        end

        errors.empty? ? { valid: true } : { valid: false, errors: errors }
      end

      # Serialize to JSON
      def to_json(*_args)
        {
          name: @name,
          created_at: @created_at,
          src: @src,
          content_type: @content_type,
          size: @size
        }.to_json
      end

      # Parse from JSON
      def self.from_json(json_str, id: nil)
        data = JSON.parse(json_str, symbolize_names: true)
        file = new(
          name: data[:name],
          src: data[:src],
          content_type: data[:content_type],
          size: data[:size],
          created_at: data[:created_at]
        )

        result = file.validate(id: id)
        raise "Validation failed: #{result[:errors].join(', ')}" unless result[:valid]

        file
      end
    end
  end
end
```

#### lib/pubky_app_specs/models/tag.rb

```ruby
# frozen_string_literal: true

module PubkyAppSpecs
  module Models
    class Tag
      include PubkyAppSpecs::IdGenerators

      MAX_LABEL_LENGTH = 20
      MIN_LABEL_LENGTH = 1
      INVALID_CHARS = [',', ':'].freeze
      PATH_SEGMENT = 'tags/'

      attr_accessor :uri, :label, :created_at

      def initialize(uri:, label:, created_at: nil)
        @uri = uri
        @label = label
        @created_at = created_at || PubkyAppSpecs.timestamp
        sanitize!
      end

      # Create a hash-based ID for this tag
      # The ID is derived from Blake3 hash of "uri:label"
      def create_id
        data = "#{@uri}:#{@label}"
        IdGenerators.create_hash_id(data)
      end

      # Generate the storage path
      def self.create_path(id)
        "#{PUBLIC_PATH}#{APP_PATH}#{PATH_SEGMENT}#{id}"
      end

      # Generate full URI
      def self.create_uri(user_id, tag_id)
        "#{PROTOCOL}#{user_id}#{create_path(tag_id)}"
      end

      # Sanitize the tag
      def sanitize!
        @label = @label.to_s.downcase
        @uri = @uri.to_s.strip
        self
      end

      # Validate the tag
      def validate(id: nil)
        errors = []

        # Validate ID if provided
        if id
          expected_id = create_id
          if id != expected_id
            errors << "Invalid ID: expected #{expected_id}, got #{id}"
          end
        end

        # Validate label length
        if @label.length < MIN_LABEL_LENGTH
          errors << "Label is too short (minimum #{MIN_LABEL_LENGTH} character)"
        elsif @label.length > MAX_LABEL_LENGTH
          errors << "Label exceeds maximum length of #{MAX_LABEL_LENGTH}"
        end

        # Validate no whitespace in label
        if @label.match?(/\s/)
          errors << 'Label cannot contain whitespace'
        end

        # Validate no invalid characters
        INVALID_CHARS.each do |char|
          if @label.include?(char)
            errors << "Label cannot contain '#{char}'"
          end
        end

        # Validate URI format
        unless Validators.valid_url?(@uri)
          errors << "Invalid URI format: #{@uri}"
        end

        errors.empty? ? { valid: true } : { valid: false, errors: errors }
      end

      # Serialize to JSON
      def to_json(*_args)
        {
          uri: @uri,
          label: @label,
          created_at: @created_at
        }.to_json
      end

      # Parse from JSON
      def self.from_json(json_str, id: nil)
        data = JSON.parse(json_str, symbolize_names: true)
        tag = new(
          uri: data[:uri],
          label: data[:label],
          created_at: data[:created_at]
        )

        result = tag.validate(id: id)
        raise "Validation failed: #{result[:errors].join(', ')}" unless result[:valid]

        tag
      end
    end
  end
end
```

#### lib/pubky_app_specs/models/bookmark.rb

```ruby
# frozen_string_literal: true

module PubkyAppSpecs
  module Models
    class Bookmark
      include PubkyAppSpecs::IdGenerators

      PATH_SEGMENT = 'bookmarks/'

      attr_accessor :uri, :created_at

      def initialize(uri:, created_at: nil)
        @uri = uri
        @created_at = created_at || PubkyAppSpecs.timestamp
        sanitize!
      end

      # Create a hash-based ID for this bookmark
      # The ID is derived from Blake3 hash of the URI
      def create_id
        IdGenerators.create_hash_id(@uri)
      end

      # Generate the storage path
      def self.create_path(id)
        "#{PUBLIC_PATH}#{APP_PATH}#{PATH_SEGMENT}#{id}"
      end

      # Generate full URI
      def self.create_uri(user_id, bookmark_id)
        "#{PROTOCOL}#{user_id}#{create_path(bookmark_id)}"
      end

      # Sanitize the bookmark
      def sanitize!
        @uri = @uri.to_s.strip
        self
      end

      # Validate the bookmark
      def validate(id: nil)
        errors = []

        # Validate ID if provided
        if id
          expected_id = create_id
          if id != expected_id
            errors << "Invalid ID: expected #{expected_id}, got #{id}"
          end
        end

        # Validate URI format
        unless Validators.valid_url?(@uri)
          errors << "Invalid URI format: #{@uri}"
        end

        errors.empty? ? { valid: true } : { valid: false, errors: errors }
      end

      # Serialize to JSON
      def to_json(*_args)
        {
          uri: @uri,
          created_at: @created_at
        }.to_json
      end

      # Parse from JSON
      def self.from_json(json_str, id: nil)
        data = JSON.parse(json_str, symbolize_names: true)
        bookmark = new(
          uri: data[:uri],
          created_at: data[:created_at]
        )

        result = bookmark.validate(id: id)
        raise "Validation failed: #{result[:errors].join(', ')}" unless result[:valid]

        bookmark
      end
    end
  end
end
```

---

## Validation Examples

### Validating a PubkyAppPost

```ruby
require_relative 'lib/pubky_app_specs/base'
require_relative 'lib/pubky_app_specs/id_generators'
require_relative 'lib/pubky_app_specs/validators'
require_relative 'lib/pubky_app_specs/models/post'

# Create a new post
post = PubkyAppSpecs::Models::Post.new(
  content: 'Hello, Pubky world! This is my first post.',
  kind: 'short'
)

# Generate an ID
post_id = post.create_id
puts "Generated Post ID: #{post_id}"  # e.g., "0033SSE3B1FQ0"

# Validate the post
result = post.validate(id: post_id)
if result[:valid]
  puts 'Post is valid!'
else
  puts "Validation errors: #{result[:errors].join(', ')}"
end

# Parse and validate from JSON
json_data = '{"content": "Hello World!", "kind": "short"}'
begin
  parsed_post = PubkyAppSpecs::Models::Post.from_json(json_data, id: post_id)
  puts "Parsed post content: #{parsed_post.content}"
rescue StandardError => e
  puts "Failed to parse: #{e.message}"
end
```

### Validating a Post with Parent (Reply)

```ruby
parent_uri = 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0'

reply = PubkyAppSpecs::Models::Post.new(
  content: 'This is a reply!',
  kind: 'short',
  parent: parent_uri
)

result = reply.validate
puts result[:valid] ? 'Reply is valid!' : "Errors: #{result[:errors]}"
```

---

## ID Generation Examples

### Generating a File ID

```ruby
require_relative 'lib/pubky_app_specs/base'
require_relative 'lib/pubky_app_specs/id_generators'
require_relative 'lib/pubky_app_specs/validators'
require_relative 'lib/pubky_app_specs/models/file'

# Create a new file
file = PubkyAppSpecs::Models::File.new(
  name: 'photo.jpg',
  src: 'pubky://user123/pub/pubky.app/blobs/ABC123',
  content_type: 'image/jpeg',
  size: 1024 * 500  # 500KB
)

# Generate timestamp-based ID
file_id = file.create_id
puts "Generated File ID: #{file_id}"  # e.g., "0033SSN7Q4EVG"

# Get the full path
path = PubkyAppSpecs::Models::File.create_path(file_id)
puts "File path: #{path}"  # "/pub/pubky.app/files/0033SSN7Q4EVG"

# Get the full URI
uri = PubkyAppSpecs::Models::File.create_uri('user123', file_id)
puts "File URI: #{uri}"  # "pubky://user123/pub/pubky.app/files/0033SSN7Q4EVG"
```

### Generating a Tag ID (Hash-based)

```ruby
require_relative 'lib/pubky_app_specs/base'
require_relative 'lib/pubky_app_specs/id_generators'
require_relative 'lib/pubky_app_specs/validators'
require_relative 'lib/pubky_app_specs/models/tag'

# Create a tag
tag = PubkyAppSpecs::Models::Tag.new(
  uri: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0',
  label: 'awesome'
)

# Generate hash-based ID
tag_id = tag.create_id
puts "Generated Tag ID: #{tag_id}"  # Hash-based, deterministic

# The same URI + label will always produce the same ID
tag2 = PubkyAppSpecs::Models::Tag.new(
  uri: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0',
  label: 'awesome'
)
puts "Same ID? #{tag.create_id == tag2.create_id}"  # true
```

### Generating a Bookmark ID

```ruby
require_relative 'lib/pubky_app_specs/base'
require_relative 'lib/pubky_app_specs/id_generators'
require_relative 'lib/pubky_app_specs/validators'
require_relative 'lib/pubky_app_specs/models/bookmark'

bookmark = PubkyAppSpecs::Models::Bookmark.new(
  uri: 'pubky://user123/pub/pubky.app/posts/0033SSE3B1FQ0'
)

bookmark_id = bookmark.create_id
puts "Generated Bookmark ID: #{bookmark_id}"
```

---

## Integration with Rails Models

### Using Custom Validators

Create a custom validator for Pubky content:

```ruby
# app/validators/pubky_post_validator.rb
class PubkyPostValidator < ActiveModel::Validator
  def validate(record)
    post = PubkyAppSpecs::Models::Post.new(
      content: record.content,
      kind: record.kind,
      parent: record.parent_uri,
      embed: record.embed_data,
      attachments: record.attachment_uris
    )

    result = post.validate(id: record.pubky_id)

    unless result[:valid]
      result[:errors].each do |error|
        record.errors.add(:base, error)
      end
    end
  end
end
```

### Rails Model Example

```ruby
# app/models/pubky_post.rb
class PubkyPost < ApplicationRecord
  include ActiveModel::Validations

  validates_with PubkyPostValidator

  before_create :generate_pubky_id

  private

  def generate_pubky_id
    self.pubky_id ||= PubkyAppSpecs::IdGenerators.create_timestamp_id
  end
end
```

### Service Object Example

```ruby
# app/services/pubky_post_creator.rb
class PubkyPostCreator
  def initialize(user_id:, content:, kind: 'short', parent: nil)
    @user_id = user_id
    @content = content
    @kind = kind
    @parent = parent
  end

  def call
    post = PubkyAppSpecs::Models::Post.new(
      content: @content,
      kind: @kind,
      parent: @parent
    )

    result = post.validate
    return { success: false, errors: result[:errors] } unless result[:valid]

    post_id = post.create_id
    path = PubkyAppSpecs::Models::Post.create_path(post_id)
    uri = PubkyAppSpecs::Models::Post.create_uri(@user_id, post_id)

    {
      success: true,
      data: {
        id: post_id,
        path: path,
        uri: uri,
        json: post.to_json
      }
    }
  end
end

# Usage:
result = PubkyPostCreator.new(
  user_id: 'user123abc',
  content: 'Hello, Pubky!',
  kind: 'short'
).call

if result[:success]
  puts "Created post with ID: #{result[:data][:id]}"
  puts "URI: #{result[:data][:uri]}"
else
  puts "Errors: #{result[:errors].join(', ')}"
end
```

---

## Complete Initializer

For convenience, here's a complete initializer that loads all modules:

```ruby
# config/initializers/pubky_app_specs.rb

# Load all PubkyAppSpecs modules
require_relative '../../lib/pubky_app_specs/base'
require_relative '../../lib/pubky_app_specs/id_generators'
require_relative '../../lib/pubky_app_specs/validators'
require_relative '../../lib/pubky_app_specs/models/post'
require_relative '../../lib/pubky_app_specs/models/file'
require_relative '../../lib/pubky_app_specs/models/tag'
require_relative '../../lib/pubky_app_specs/models/bookmark'

# Make modules globally accessible
Rails.application.config.pubky_specs = PubkyAppSpecs
```

---

## URI Helpers

```ruby
# lib/pubky_app_specs/uri_helpers.rb
module PubkyAppSpecs
  module UriHelpers
    module_function

    def base_uri(user_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}"
    end

    def user_uri(user_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}profile.json"
    end

    def post_uri(user_id, post_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}posts/#{post_id}"
    end

    def file_uri(user_id, file_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}files/#{file_id}"
    end

    def tag_uri(user_id, tag_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}tags/#{tag_id}"
    end

    def bookmark_uri(user_id, bookmark_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}bookmarks/#{bookmark_id}"
    end

    def follow_uri(user_id, followee_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}follows/#{followee_id}"
    end

    def blob_uri(user_id, blob_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}blobs/#{blob_id}"
    end

    def feed_uri(user_id, feed_id)
      "#{PROTOCOL}#{user_id}#{PUBLIC_PATH}#{APP_PATH}feeds/#{feed_id}"
    end
  end
end
```

---

## Testing

Here's an example RSpec test:

```ruby
# spec/lib/pubky_app_specs/models/post_spec.rb
require 'rails_helper'

RSpec.describe PubkyAppSpecs::Models::Post do
  describe '#create_id' do
    it 'generates a 13-character ID' do
      post = described_class.new(content: 'Hello')
      expect(post.create_id.length).to eq(13)
    end
  end

  describe '#validate' do
    context 'with valid content' do
      it 'returns valid: true' do
        post = described_class.new(content: 'Hello, world!')
        result = post.validate
        expect(result[:valid]).to be true
      end
    end

    context 'with content exceeding max length' do
      it 'truncates content during sanitization' do
        long_content = 'x' * 3000
        post = described_class.new(content: long_content, kind: 'short')
        expect(post.content.length).to eq(2000)
      end
    end

    context 'with reserved content [DELETED]' do
      it 'replaces with "empty"' do
        post = described_class.new(content: '[DELETED]')
        expect(post.content).to eq('empty')
      end
    end
  end
end
```

---

## Notes

1. **Blake3 Hashing**: The Ruby `blake3` gem is required for hash-based ID generation. Install it with `gem install blake3` or add it to your Gemfile.

2. **Crockford Base32**: The implementation above provides a custom Crockford Base32 encoder/decoder. You can also use the `base32` gem with custom alphabet configuration.

3. **Timestamp Precision**: IDs use microsecond precision timestamps. Ensure your system clock is accurate.

4. **Validation Rules**: The validation rules match those in the Rust implementation. See the main README for detailed specifications.

5. **Thread Safety**: The ID generation functions are thread-safe as they don't share mutable state.

---

## License

This integration guide follows the MIT License of the pubky-app-specs project.
