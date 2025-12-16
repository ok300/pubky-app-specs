//! FFI (Foreign Function Interface) module for using pubky-app-specs from non-Rust languages.
//!
//! This module provides C-compatible functions that can be called from Ruby, Python, and other
//! languages via FFI. All functions use JSON strings for input/output to simplify cross-language
//! data marshalling.
//!
//! # Memory Management
//!
//! All functions that return strings allocate memory that must be freed by the caller using
//! `pubky_free_string`. Failure to free returned strings will result in memory leaks.
//!
//! # Thread Safety
//!
//! All functions are thread-safe and can be called from multiple threads concurrently.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::models::bookmark::PubkyAppBookmark;
use crate::models::file::PubkyAppFile;
use crate::models::post::{PubkyAppPost, PubkyAppPostKind};
use crate::models::tag::PubkyAppTag;
use crate::models::user::{PubkyAppUser, PubkyAppUserLink};
use crate::traits::{HasIdPath, HasPath, HashId, TimestampId, Validatable};

/// Helper to convert a C string pointer to a Rust String.
///
/// # Safety
/// The pointer must be a valid null-terminated C string.
unsafe fn c_str_to_string(ptr: *const c_char) -> Result<String, &'static str> {
    if ptr.is_null() {
        return Err("Input string pointer is null");
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| "Invalid UTF-8 in input string")
}

/// Helper to convert a Rust String to a C string pointer.
fn string_to_c_str(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Helper to create an error JSON response.
fn error_json(message: &str) -> *mut c_char {
    let json = format!(r#"{{"success":false,"error":"{}"}}"#, message.replace('"', "\\\""));
    string_to_c_str(json)
}

// =============================================================================
// Memory Management
// =============================================================================

/// Free a string allocated by this library.
///
/// # Safety
/// The pointer must be a valid CString allocated by one of the functions in this module.
/// After calling this function, the pointer is invalid and must not be used.
#[no_mangle]
pub unsafe extern "C" fn pubky_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

// =============================================================================
// PubkyAppPost Functions
// =============================================================================

/// Create and validate a new post.
///
/// # Arguments
/// * `content` - The post content (null-terminated C string)
/// * `kind` - The post kind: "short", "long", "image", "video", "link", or "file"
///
/// # Returns
/// A JSON string with the result. On success:
/// ```json
/// {
///   "success": true,
///   "id": "0033SSE3B1FQ0",
///   "path": "/pub/pubky.app/posts/0033SSE3B1FQ0",
///   "post": { "content": "...", "kind": "short", ... }
/// }
/// ```
///
/// On failure:
/// ```json
/// { "success": false, "error": "Error message" }
/// ```
///
/// # Safety
/// Both `content` and `kind` must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_post(
    content: *const c_char,
    kind: *const c_char,
) -> *mut c_char {
    let content = match c_str_to_string(content) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let kind_str = match c_str_to_string(kind) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let kind = match kind_str.as_str() {
        "short" => PubkyAppPostKind::Short,
        "long" => PubkyAppPostKind::Long,
        "image" => PubkyAppPostKind::Image,
        "video" => PubkyAppPostKind::Video,
        "link" => PubkyAppPostKind::Link,
        "file" => PubkyAppPostKind::File,
        _ => return error_json(&format!("Invalid post kind: {}", kind_str)),
    };

    let post = PubkyAppPost::new(content, kind, None, None, None);
    let post_id = post.create_id();

    if let Err(e) = post.validate(Some(&post_id)) {
        return error_json(&e);
    }

    let path = PubkyAppPost::create_path(&post_id);
    let post_json = match serde_json::to_value(&post) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "id": post_id,
        "path": path,
        "post": post_json
    });

    string_to_c_str(result.to_string())
}

/// Create and validate a new post with all optional fields.
///
/// # Arguments
/// * `content` - The post content
/// * `kind` - The post kind
/// * `parent` - Optional parent URI (for replies), or null
/// * `embed_uri` - Optional embed URI, or null
/// * `embed_kind` - Optional embed kind, or null (required if embed_uri is provided)
///
/// # Safety
/// All non-null pointers must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_post_full(
    content: *const c_char,
    kind: *const c_char,
    parent: *const c_char,
    embed_uri: *const c_char,
    embed_kind: *const c_char,
) -> *mut c_char {
    let content = match c_str_to_string(content) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let kind_str = match c_str_to_string(kind) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let kind = match kind_str.as_str() {
        "short" => PubkyAppPostKind::Short,
        "long" => PubkyAppPostKind::Long,
        "image" => PubkyAppPostKind::Image,
        "video" => PubkyAppPostKind::Video,
        "link" => PubkyAppPostKind::Link,
        "file" => PubkyAppPostKind::File,
        _ => return error_json(&format!("Invalid post kind: {}", kind_str)),
    };

    let parent_opt = if parent.is_null() {
        None
    } else {
        match c_str_to_string(parent) {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };

    let embed_opt = if embed_uri.is_null() {
        None
    } else {
        match (c_str_to_string(embed_uri), c_str_to_string(embed_kind)) {
            (Ok(uri), Ok(ek_str)) if !uri.is_empty() => {
                let ek = match ek_str.as_str() {
                    "short" => PubkyAppPostKind::Short,
                    "long" => PubkyAppPostKind::Long,
                    "image" => PubkyAppPostKind::Image,
                    "video" => PubkyAppPostKind::Video,
                    "link" => PubkyAppPostKind::Link,
                    "file" => PubkyAppPostKind::File,
                    _ => return error_json(&format!("Invalid embed kind: {}", ek_str)),
                };
                Some(crate::models::post::PubkyAppPostEmbed { uri, kind: ek })
            }
            _ => None,
        }
    };

    let post = PubkyAppPost::new(content, kind, parent_opt, embed_opt, None);
    let post_id = post.create_id();

    if let Err(e) = post.validate(Some(&post_id)) {
        return error_json(&e);
    }

    let path = PubkyAppPost::create_path(&post_id);
    let post_json = match serde_json::to_value(&post) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "id": post_id,
        "path": path,
        "post": post_json
    });

    string_to_c_str(result.to_string())
}

/// Validate a post from JSON.
///
/// # Arguments
/// * `json_str` - JSON string representation of a post
/// * `id` - Optional post ID to validate against, or null
///
/// # Returns
/// JSON string: `{"valid": true}` or `{"valid": false, "error": "..."}`
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_post(
    json_str: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let json = match c_str_to_string(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };

    let id_opt = if id.is_null() {
        None
    } else {
        c_str_to_string(id).ok()
    };

    match serde_json::from_str::<PubkyAppPost>(&json) {
        Ok(post) => {
            let post = post.sanitize();
            match post.validate(id_opt.as_deref()) {
                Ok(_) => string_to_c_str(r#"{"valid":true}"#.to_string()),
                Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.replace('"', "\\\""))),
            }
        }
        Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.to_string().replace('"', "\\\""))),
    }
}

/// Generate a timestamp-based ID (for posts and files).
///
/// # Returns
/// A 13-character Crockford Base32 encoded timestamp ID.
///
/// # Safety
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub extern "C" fn pubky_generate_timestamp_id() -> *mut c_char {
    // Generate timestamp ID using the same logic as TimestampId trait
    let now = crate::common::timestamp();
    let bytes = now.to_be_bytes();
    let id = base32::encode(base32::Alphabet::Crockford, &bytes);
    string_to_c_str(id)
}

// =============================================================================
// PubkyAppFile Functions
// =============================================================================

/// Create and validate a new file.
///
/// # Arguments
/// * `name` - The file name
/// * `src` - The file source URI (typically a pubky:// blob URI)
/// * `content_type` - MIME type of the file (e.g., "image/png")
/// * `size` - File size in bytes
///
/// # Returns
/// JSON string with `success`, `id`, `path`, and `file` fields.
///
/// # Safety
/// All string pointers must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_file(
    name: *const c_char,
    src: *const c_char,
    content_type: *const c_char,
    size: usize,
) -> *mut c_char {
    let name = match c_str_to_string(name) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let src = match c_str_to_string(src) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let content_type = match c_str_to_string(content_type) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let file = PubkyAppFile::new(name, src, content_type, size);
    let file_id = file.create_id();

    if let Err(e) = file.validate(Some(&file_id)) {
        return error_json(&e);
    }

    let path = PubkyAppFile::create_path(&file_id);
    let file_json = match serde_json::to_value(&file) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "id": file_id,
        "path": path,
        "file": file_json
    });

    string_to_c_str(result.to_string())
}

/// Validate a file from JSON.
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_file(
    json_str: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let json = match c_str_to_string(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };

    let id_opt = if id.is_null() {
        None
    } else {
        c_str_to_string(id).ok()
    };

    match serde_json::from_str::<PubkyAppFile>(&json) {
        Ok(file) => {
            let file = file.sanitize();
            match file.validate(id_opt.as_deref()) {
                Ok(_) => string_to_c_str(r#"{"valid":true}"#.to_string()),
                Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.replace('"', "\\\""))),
            }
        }
        Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.to_string().replace('"', "\\\""))),
    }
}

// =============================================================================
// PubkyAppBookmark Functions
// =============================================================================

/// Create and validate a new bookmark.
///
/// # Arguments
/// * `uri` - The URI to bookmark
///
/// # Returns
/// JSON string with `success`, `id`, `path`, and `bookmark` fields.
///
/// # Safety
/// `uri` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_bookmark(uri: *const c_char) -> *mut c_char {
    let uri = match c_str_to_string(uri) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let bookmark = PubkyAppBookmark::new(uri);
    let bookmark_id = bookmark.create_id();

    if let Err(e) = bookmark.validate(Some(&bookmark_id)) {
        return error_json(&e);
    }

    let path = PubkyAppBookmark::create_path(&bookmark_id);
    let bookmark_json = match serde_json::to_value(&bookmark) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "id": bookmark_id,
        "path": path,
        "bookmark": bookmark_json
    });

    string_to_c_str(result.to_string())
}

/// Validate a bookmark from JSON.
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_bookmark(
    json_str: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let json = match c_str_to_string(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };

    let id_opt = if id.is_null() {
        None
    } else {
        c_str_to_string(id).ok()
    };

    match serde_json::from_str::<PubkyAppBookmark>(&json) {
        Ok(bookmark) => {
            let bookmark = bookmark.sanitize();
            match bookmark.validate(id_opt.as_deref()) {
                Ok(_) => string_to_c_str(r#"{"valid":true}"#.to_string()),
                Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.replace('"', "\\\""))),
            }
        }
        Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.to_string().replace('"', "\\\""))),
    }
}

// =============================================================================
// PubkyAppTag Functions
// =============================================================================

/// Create and validate a new tag.
///
/// # Arguments
/// * `uri` - The URI to tag
/// * `label` - The tag label
///
/// # Returns
/// JSON string with `success`, `id`, `path`, and `tag` fields.
///
/// # Safety
/// Both pointers must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_tag(
    uri: *const c_char,
    label: *const c_char,
) -> *mut c_char {
    let uri = match c_str_to_string(uri) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let label = match c_str_to_string(label) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let tag = PubkyAppTag::new(uri, label);
    let tag_id = tag.create_id();

    if let Err(e) = tag.validate(Some(&tag_id)) {
        return error_json(&e);
    }

    let path = PubkyAppTag::create_path(&tag_id);
    let tag_json = match serde_json::to_value(&tag) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "id": tag_id,
        "path": path,
        "tag": tag_json
    });

    string_to_c_str(result.to_string())
}

/// Validate a tag from JSON.
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_tag(
    json_str: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let json = match c_str_to_string(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };

    let id_opt = if id.is_null() {
        None
    } else {
        c_str_to_string(id).ok()
    };

    match serde_json::from_str::<PubkyAppTag>(&json) {
        Ok(tag) => {
            let tag = tag.sanitize();
            match tag.validate(id_opt.as_deref()) {
                Ok(_) => string_to_c_str(r#"{"valid":true}"#.to_string()),
                Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.replace('"', "\\\""))),
            }
        }
        Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.to_string().replace('"', "\\\""))),
    }
}

// =============================================================================
// PubkyAppUser Functions
// =============================================================================

/// Create and validate a new user profile.
///
/// # Arguments
/// * `name` - User's display name
/// * `bio` - Optional bio (null-terminated string or null)
/// * `image` - Optional image URL (null-terminated string or null)
/// * `status` - Optional status message (null-terminated string or null)
///
/// # Returns
/// JSON string with `success`, `path`, and `user` fields.
///
/// # Safety
/// All non-null pointers must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_user(
    name: *const c_char,
    bio: *const c_char,
    image: *const c_char,
    status: *const c_char,
) -> *mut c_char {
    let name = match c_str_to_string(name) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let bio_opt = if bio.is_null() {
        None
    } else {
        c_str_to_string(bio).ok().filter(|s| !s.is_empty())
    };

    let image_opt = if image.is_null() {
        None
    } else {
        c_str_to_string(image).ok().filter(|s| !s.is_empty())
    };

    let status_opt = if status.is_null() {
        None
    } else {
        c_str_to_string(status).ok().filter(|s| !s.is_empty())
    };

    let user = PubkyAppUser::new(name, bio_opt, image_opt, None, status_opt);

    if let Err(e) = user.validate(None) {
        return error_json(&e);
    }

    let path = PubkyAppUser::create_path();
    let user_json = match serde_json::to_value(&user) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "path": path,
        "user": user_json
    });

    string_to_c_str(result.to_string())
}

/// Create a user profile with links.
///
/// # Arguments
/// * `name` - User's display name
/// * `bio` - Optional bio
/// * `image` - Optional image URL
/// * `status` - Optional status message
/// * `links_json` - JSON array of links, e.g. `[{"title":"GitHub","url":"https://github.com/user"}]`
///
/// # Safety
/// All non-null pointers must be valid null-terminated C strings.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_create_user_with_links(
    name: *const c_char,
    bio: *const c_char,
    image: *const c_char,
    status: *const c_char,
    links_json: *const c_char,
) -> *mut c_char {
    let name = match c_str_to_string(name) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let bio_opt = if bio.is_null() {
        None
    } else {
        c_str_to_string(bio).ok().filter(|s| !s.is_empty())
    };

    let image_opt = if image.is_null() {
        None
    } else {
        c_str_to_string(image).ok().filter(|s| !s.is_empty())
    };

    let status_opt = if status.is_null() {
        None
    } else {
        c_str_to_string(status).ok().filter(|s| !s.is_empty())
    };

    let links_opt = if links_json.is_null() {
        None
    } else {
        match c_str_to_string(links_json) {
            Ok(json) => {
                serde_json::from_str::<Vec<PubkyAppUserLink>>(&json).ok()
            }
            Err(_) => None,
        }
    };

    let user = PubkyAppUser::new(name, bio_opt, image_opt, links_opt, status_opt);

    if let Err(e) = user.validate(None) {
        return error_json(&e);
    }

    let path = PubkyAppUser::create_path();
    let user_json = match serde_json::to_value(&user) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let result = serde_json::json!({
        "success": true,
        "path": path,
        "user": user_json
    });

    string_to_c_str(result.to_string())
}

/// Validate a user profile from JSON.
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_validate_user(json_str: *const c_char) -> *mut c_char {
    let json = match c_str_to_string(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };

    match serde_json::from_str::<PubkyAppUser>(&json) {
        Ok(user) => {
            let user = user.sanitize();
            match user.validate(None) {
                Ok(_) => string_to_c_str(r#"{"valid":true}"#.to_string()),
                Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.replace('"', "\\\""))),
            }
        }
        Err(e) => string_to_c_str(format!(r#"{{"valid":false,"error":"{}"}}"#, e.to_string().replace('"', "\\\""))),
    }
}

// =============================================================================
// Path Helper Functions
// =============================================================================

/// Get the path for a post given its ID.
///
/// # Safety
/// `id` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_get_post_path(id: *const c_char) -> *mut c_char {
    let id = match c_str_to_string(id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    string_to_c_str(PubkyAppPost::create_path(&id))
}

/// Get the path for a file given its ID.
///
/// # Safety
/// `id` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_get_file_path(id: *const c_char) -> *mut c_char {
    let id = match c_str_to_string(id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    string_to_c_str(PubkyAppFile::create_path(&id))
}

/// Get the path for a bookmark given its ID.
///
/// # Safety
/// `id` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_get_bookmark_path(id: *const c_char) -> *mut c_char {
    let id = match c_str_to_string(id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    string_to_c_str(PubkyAppBookmark::create_path(&id))
}

/// Get the path for a tag given its ID.
///
/// # Safety
/// `id` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_get_tag_path(id: *const c_char) -> *mut c_char {
    let id = match c_str_to_string(id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    string_to_c_str(PubkyAppTag::create_path(&id))
}

/// Get the path for a user profile.
///
/// # Safety
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub extern "C" fn pubky_get_user_path() -> *mut c_char {
    string_to_c_str(PubkyAppUser::create_path())
}

// =============================================================================
// URI Parsing Functions
// =============================================================================

/// Parse a pubky:// URI and extract user_id and resource information.
///
/// # Arguments
/// * `uri` - A pubky:// URI string (e.g., "pubky://user123/pub/pubky.app/posts/abc123")
///
/// # Returns
/// A JSON string with the parsed URI information. On success:
/// ```json
/// {
///   "success": true,
///   "user_id": "user123...",
///   "resource_type": "posts",
///   "resource_id": "abc123"
/// }
/// ```
///
/// For resources without an ID (like User or LastRead):
/// ```json
/// {
///   "success": true,
///   "user_id": "user123...",
///   "resource_type": "profile.json",
///   "resource_id": null
/// }
/// ```
///
/// On failure:
/// ```json
/// { "success": false, "error": "Error message" }
/// ```
///
/// # Safety
/// `uri` must be a valid null-terminated C string.
/// The returned string must be freed with `pubky_free_string`.
#[no_mangle]
pub unsafe extern "C" fn pubky_parse_uri(uri: *const c_char) -> *mut c_char {
    let uri_str = match c_str_to_string(uri) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    match crate::ParsedUri::try_from(uri_str.as_str()) {
        Ok(parsed) => {
            let result = serde_json::json!({
                "success": true,
                "user_id": parsed.user_id.to_string(),
                "resource_type": parsed.resource.to_string(),
                "resource_id": parsed.resource.id()
            });

            string_to_c_str(result.to_string())
        }
        Err(e) => error_json(&e),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_create_post() {
        let content = CString::new("Hello, world!").unwrap();
        let kind = CString::new("short").unwrap();

        unsafe {
            let result_ptr = pubky_create_post(content.as_ptr(), kind.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert!(result["id"].is_string());
            assert!(result["path"].is_string());
            assert!(result["post"]["content"].is_string());

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_validate_post() {
        let json = CString::new(r#"{"content":"Hello","kind":"short"}"#).unwrap();

        unsafe {
            let result_ptr = pubky_validate_post(json.as_ptr(), std::ptr::null());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["valid"], true);

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_create_file() {
        let name = CString::new("test.png").unwrap();
        let src = CString::new("pubky://user123/pub/pubky.app/blobs/abc123").unwrap();
        let content_type = CString::new("image/png").unwrap();

        unsafe {
            let result_ptr = pubky_create_file(
                name.as_ptr(),
                src.as_ptr(),
                content_type.as_ptr(),
                1024,
            );
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert!(result["id"].is_string());

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_create_bookmark() {
        let uri = CString::new("pubky://user123/pub/pubky.app/posts/abc123").unwrap();

        unsafe {
            let result_ptr = pubky_create_bookmark(uri.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert!(result["id"].is_string());

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_create_tag() {
        let uri = CString::new("pubky://user123/pub/pubky.app/posts/abc123").unwrap();
        let label = CString::new("cool").unwrap();

        unsafe {
            let result_ptr = pubky_create_tag(uri.as_ptr(), label.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert!(result["id"].is_string());

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_create_user() {
        let name = CString::new("Alice").unwrap();
        let bio = CString::new("Developer").unwrap();

        unsafe {
            let result_ptr = pubky_create_user(
                name.as_ptr(),
                bio.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            );
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert!(result["path"].is_string());
            assert_eq!(result["user"]["name"], "Alice");

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_generate_timestamp_id() {
        let id_ptr = pubky_generate_timestamp_id();
        assert!(!id_ptr.is_null());

        unsafe {
            let id = CStr::from_ptr(id_ptr).to_str().unwrap();
            assert_eq!(id.len(), 13);
            pubky_free_string(id_ptr);
        }
    }

    #[test]
    fn test_get_paths() {
        let id = CString::new("0033SSE3B1FQ0").unwrap();

        unsafe {
            let post_path = pubky_get_post_path(id.as_ptr());
            let file_path = pubky_get_file_path(id.as_ptr());
            let bookmark_path = pubky_get_bookmark_path(id.as_ptr());
            let tag_path = pubky_get_tag_path(id.as_ptr());
            let user_path = pubky_get_user_path();

            assert!(!post_path.is_null());
            assert!(!file_path.is_null());
            assert!(!bookmark_path.is_null());
            assert!(!tag_path.is_null());
            assert!(!user_path.is_null());

            let post_path_str = CStr::from_ptr(post_path).to_str().unwrap();
            assert!(post_path_str.contains("posts/"));

            pubky_free_string(post_path);
            pubky_free_string(file_path);
            pubky_free_string(bookmark_path);
            pubky_free_string(tag_path);
            pubky_free_string(user_path);
        }
    }

    #[test]
    fn test_parse_uri_post() {
        let uri = CString::new("pubky://operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo/pub/pubky.app/posts/0033SSE3B1FQ0").unwrap();

        unsafe {
            let result_ptr = pubky_parse_uri(uri.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert_eq!(result["user_id"], "operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo");
            assert_eq!(result["resource_type"], "posts");
            assert_eq!(result["resource_id"], "0033SSE3B1FQ0");

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_parse_uri_user() {
        let uri = CString::new("pubky://operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo/pub/pubky.app/profile.json").unwrap();

        unsafe {
            let result_ptr = pubky_parse_uri(uri.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], true);
            assert_eq!(result["resource_type"], "profile.json");
            assert!(result["resource_id"].is_null());

            pubky_free_string(result_ptr);
        }
    }

    #[test]
    fn test_parse_uri_invalid() {
        let uri = CString::new("http://invalid/uri").unwrap();

        unsafe {
            let result_ptr = pubky_parse_uri(uri.as_ptr());
            assert!(!result_ptr.is_null());

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            let result: serde_json::Value = serde_json::from_str(result_str).unwrap();

            assert_eq!(result["success"], false);
            assert!(result["error"].is_string());

            pubky_free_string(result_ptr);
        }
    }
}
