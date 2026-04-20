use crate::{
    traits::{HasIdPath, HasPath},
    PubkyAppBlob, PubkyAppBookmark, PubkyAppFeed, PubkyAppFile, PubkyAppFollow, PubkyAppLastRead,
    PubkyAppMute, PubkyAppPost, PubkyAppTag, PubkyAppUser, PubkyId, APP_PATH, PROTOCOL,
    PUBLIC_PATH,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use url::Url;

#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub enum Resource {
    User,
    Post(String),
    Follow(PubkyId),
    Mute(PubkyId),
    Bookmark(String),
    Tag(String),
    File(String),
    Blob(String),
    Feed(String),
    LastRead,
    #[default]
    Unknown,
}

impl fmt::Display for Resource {
    /// Returns the resource name without any identifier.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the associated constant for each resource type, trimming any trailing '/'
        let name = match self {
            Resource::User => PubkyAppUser::PATH_SEGMENT.trim_end_matches('/'),
            Resource::LastRead => PubkyAppLastRead::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Post(_) => PubkyAppPost::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Follow(_) => PubkyAppFollow::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Mute(_) => PubkyAppMute::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Bookmark(_) => PubkyAppBookmark::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Tag(_) => PubkyAppTag::PATH_SEGMENT.trim_end_matches('/'),
            Resource::File(_) => PubkyAppFile::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Blob(_) => PubkyAppBlob::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Feed(_) => PubkyAppFeed::PATH_SEGMENT.trim_end_matches('/'),
            Resource::Unknown => "unknown",
        };
        write!(f, "{}", name)
    }
}

impl Resource {
    /// Returns the identifier as a `Some(String)` if the resource variant holds one,
    /// or `None` if there is no identifier.
    pub fn id(&self) -> Option<String> {
        match self {
            Resource::Post(id) => Some(id.clone()),
            Resource::Follow(id) => Some(id.to_string()),
            Resource::Mute(id) => Some(id.to_string()),
            Resource::Bookmark(id) => Some(id.clone()),
            Resource::Tag(id) => Some(id.clone()),
            Resource::File(id) => Some(id.clone()),
            Resource::Blob(id) => Some(id.clone()),
            Resource::Feed(id) => Some(id.clone()),
            // The following variants do not carry an id.
            Resource::User | Resource::LastRead | Resource::Unknown => None,
        }
    }
}

#[deprecated(note = "Use `ParsedUriV2` instead")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ParsedUri {
    pub user_id: PubkyId,
    pub resource: Resource,
}

/// A parsed Pubky URI that supports both standard `pubky.app` paths and
/// universal tag URIs from other applications.
///
/// # Variants
///
/// - [`AppSpec`](ParsedUriV2::AppSpec) — standard `pubky.app` URIs of the form
///   `pubky://<user_id>/pub/pubky.app/<resource>[/<id>]`
///
/// - [`UniversalTag`](ParsedUriV2::UniversalTag) — tag URIs from a different app of the form
///   `pubky://<user_id>/pub/<app>/tags/<tag_id>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParsedUriV2 {
    /// Standard URIs (pubky.app path)
    AppSpec {
        user_id: PubkyId,
        resource: Resource,
    },
    /// Universal tag URI from a different app.
    /// Format: pubky://<user_id>/pub/<app>/tags/<tag_id>
    UniversalTag {
        user_id: PubkyId,
        app: String,
        resource: Resource,
        tag_id: String,
    },
}

#[allow(deprecated)]
impl ParsedUri {
    /// Converts the [ParsedUri] back into its URI string representation.
    /// Returns an error if the resource is Unknown.
    pub fn try_to_uri_str(&self) -> Result<String, String> {
        use crate::traits::{HasIdPath, HasPath};

        let path = match &self.resource {
            Resource::User => PubkyAppUser::create_path(),
            Resource::LastRead => PubkyAppLastRead::create_path(),
            Resource::Post(id) => PubkyAppPost::create_path(id),
            Resource::Follow(id) => PubkyAppFollow::create_path(id.as_ref()),
            Resource::Mute(id) => PubkyAppMute::create_path(id.as_ref()),
            Resource::Bookmark(id) => PubkyAppBookmark::create_path(id),
            Resource::Tag(id) => PubkyAppTag::create_path(id),
            Resource::File(id) => PubkyAppFile::create_path(id),
            Resource::Blob(id) => PubkyAppBlob::create_path(id),
            Resource::Feed(id) => PubkyAppFeed::create_path(id),
            Resource::Unknown => return Err("Cannot convert Unknown resource to URI".to_string()),
        };

        Ok([PROTOCOL, self.user_id.as_ref(), &path].concat())
    }
}

impl ParsedUriV2 {
    /// Returns the user ID for any variant.
    pub fn user_id(&self) -> &PubkyId {
        match self {
            ParsedUriV2::AppSpec { user_id, .. } => user_id,
            ParsedUriV2::UniversalTag { user_id, .. } => user_id,
        }
    }

    /// Returns the [Resource] for any variant.
    pub fn resource(&self) -> &Resource {
        match self {
            ParsedUriV2::AppSpec { resource, .. } => resource,
            ParsedUriV2::UniversalTag { resource, .. } => resource,
        }
    }

    /// Converts the [ParsedUriV2] back into its URI string representation.
    /// Returns an error if the resource is Unknown.
    pub fn try_to_uri_str(&self) -> Result<String, String> {
        use crate::traits::{HasIdPath, HasPath};

        match self {
            ParsedUriV2::AppSpec { user_id, resource } => {
                let path = match resource {
                    Resource::User => PubkyAppUser::create_path(),
                    Resource::LastRead => PubkyAppLastRead::create_path(),
                    Resource::Post(id) => PubkyAppPost::create_path(id),
                    Resource::Follow(id) => PubkyAppFollow::create_path(id.as_ref()),
                    Resource::Mute(id) => PubkyAppMute::create_path(id.as_ref()),
                    Resource::Bookmark(id) => PubkyAppBookmark::create_path(id),
                    Resource::Tag(id) => PubkyAppTag::create_path(id),
                    Resource::File(id) => PubkyAppFile::create_path(id),
                    Resource::Blob(id) => PubkyAppBlob::create_path(id),
                    Resource::Feed(id) => PubkyAppFeed::create_path(id),
                    Resource::Unknown => {
                        return Err("Cannot convert Unknown resource to URI".to_string())
                    }
                };
                Ok([PROTOCOL, user_id.as_ref(), &path].concat())
            }
            ParsedUriV2::UniversalTag {
                user_id,
                app,
                tag_id,
                ..
            } => Ok(format!(
                "{}{}{}{}/tags/{}",
                PROTOCOL,
                user_id.as_ref(),
                PUBLIC_PATH,
                app,
                tag_id
            )),
        }
    }
}

#[allow(deprecated)]
impl TryFrom<&str> for ParsedUri {
    type Error = String;
    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        // 0. Validate and sanitize the URL.
        let parsed_url = Url::parse(uri).map_err(|e| format!("Invalid URL: {}", e))?;

        // 1. Validate the scheme (using PROTOCOL without the "://")
        if parsed_url.scheme() != PROTOCOL.trim_end_matches("://") {
            return Err(format!(
                "Invalid URI, must start with '{}': {}",
                PROTOCOL, uri
            ));
        }

        // 2. Extract the user_id from the host.
        let user_id_str = parsed_url
            .host_str()
            .ok_or_else(|| format!("Missing user ID in URI: {}", uri))?;
        let user_id = PubkyId::try_from(user_id_str)?;

        // 3. Get the path segments.
        // Expected URI structure:
        // pubky://<user_id>/<public>/<app>/<resource>[/<id>]
        let segments: Vec<&str> = parsed_url
            .path_segments()
            .ok_or_else(|| format!("Cannot parse path segments from URI: {}", uri))?
            .collect();
        if segments.len() < 2 {
            return Err(format!("Not enough path segments in URI: {}", uri));
        }
        if segments[0] != PUBLIC_PATH.trim_matches('/') {
            return Err(format!(
                "Expected public path '{}' but got '{}' in URI: {}",
                PUBLIC_PATH, segments[0], uri
            ));
        }
        if segments[1] != APP_PATH.trim_matches('/') {
            return Err(format!(
                "Expected app path '{}' but got '{}' in URI: {}",
                APP_PATH, segments[1], uri
            ));
        }

        // 4. Determine the resource from the remaining segments.
        let resource = match segments[2..] {
            // No extra segments.
            [] => Resource::Unknown,
            // A single segment: must exactly match an identifier-less route.
            [segment] => match segment {
                PubkyAppUser::PATH_SEGMENT => Resource::User,
                PubkyAppLastRead::PATH_SEGMENT => Resource::LastRead,
                _ => Resource::Unknown,
            },
            // Two or more segments and the id is not empty.
            [res_type, id, ..] if !id.is_empty() => {
                let resource_type = format!("{}/", res_type);
                match resource_type.as_str() {
                    PubkyAppPost::PATH_SEGMENT => Resource::Post(id.to_string()),
                    PubkyAppFollow::PATH_SEGMENT => PubkyId::try_from(id).map(Resource::Follow)?,
                    PubkyAppMute::PATH_SEGMENT => PubkyId::try_from(id).map(Resource::Mute)?,
                    PubkyAppBookmark::PATH_SEGMENT => Resource::Bookmark(id.to_string()),
                    PubkyAppTag::PATH_SEGMENT => Resource::Tag(id.to_string()),
                    PubkyAppFile::PATH_SEGMENT => Resource::File(id.to_string()),
                    PubkyAppBlob::PATH_SEGMENT => Resource::Blob(id.to_string()),
                    PubkyAppFeed::PATH_SEGMENT => Resource::Feed(id.to_string()),
                    _ => Resource::Unknown,
                }
            }
            // If the identifier is empty or doesn't match the expected pattern.
            _ => Resource::Unknown,
        };

        Ok(ParsedUri { user_id, resource })
    }
}

#[allow(deprecated)]
impl TryFrom<String> for ParsedUri {
    type Error = String;

    fn try_from(uri: String) -> Result<Self, Self::Error> {
        ParsedUri::try_from(uri.as_str())
    }
}

impl TryFrom<&str> for ParsedUriV2 {
    type Error = String;

    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        // 0. Validate and sanitize the URL.
        let parsed_url = Url::parse(uri).map_err(|e| format!("Invalid URL: {}", e))?;

        // 1. Validate the scheme (using PROTOCOL without the "://")
        if parsed_url.scheme() != PROTOCOL.trim_end_matches("://") {
            return Err(format!(
                "Invalid URI, must start with '{}': {}",
                PROTOCOL, uri
            ));
        }

        // 2. Extract the user_id from the host.
        let user_id_str = parsed_url
            .host_str()
            .ok_or_else(|| format!("Missing user ID in URI: {}", uri))?;
        let user_id = PubkyId::try_from(user_id_str)?;

        // 3. Get the path segments.
        let segments: Vec<&str> = parsed_url
            .path_segments()
            .ok_or_else(|| format!("Cannot parse path segments from URI: {}", uri))?
            .collect();
        if segments.len() < 2 {
            return Err(format!("Not enough path segments in URI: {}", uri));
        }
        if segments[0] != PUBLIC_PATH.trim_matches('/') {
            return Err(format!(
                "Expected public path '{}' but got '{}' in URI: {}",
                PUBLIC_PATH, segments[0], uri
            ));
        }

        let app_segment = segments[1];

        // 4. Determine whether this is a standard pubky.app URI or a universal tag URI.
        if app_segment == APP_PATH.trim_matches('/') {
            // Standard pubky.app URI
            let resource = match segments[2..] {
                [] => Resource::Unknown,
                [segment] => match segment {
                    PubkyAppUser::PATH_SEGMENT => Resource::User,
                    PubkyAppLastRead::PATH_SEGMENT => Resource::LastRead,
                    _ => Resource::Unknown,
                },
                [res_type, id, ..] if !id.is_empty() => {
                    let resource_type = format!("{}/", res_type);
                    match resource_type.as_str() {
                        PubkyAppPost::PATH_SEGMENT => Resource::Post(id.to_string()),
                        PubkyAppFollow::PATH_SEGMENT => {
                            PubkyId::try_from(id).map(Resource::Follow)?
                        }
                        PubkyAppMute::PATH_SEGMENT => PubkyId::try_from(id).map(Resource::Mute)?,
                        PubkyAppBookmark::PATH_SEGMENT => Resource::Bookmark(id.to_string()),
                        PubkyAppTag::PATH_SEGMENT => Resource::Tag(id.to_string()),
                        PubkyAppFile::PATH_SEGMENT => Resource::File(id.to_string()),
                        PubkyAppBlob::PATH_SEGMENT => Resource::Blob(id.to_string()),
                        PubkyAppFeed::PATH_SEGMENT => Resource::Feed(id.to_string()),
                        _ => Resource::Unknown,
                    }
                }
                _ => Resource::Unknown,
            };

            Ok(ParsedUriV2::AppSpec { user_id, resource })
        } else {
            // Non-pubky.app path — only recognize universal tag URIs.
            // Expected: pubky://<user_id>/pub/<app>/tags/<tag_id>
            match segments[2..] {
                [res_type, tag_id, ..] if !tag_id.is_empty() && res_type == "tags" => {
                    let tag_id = tag_id.to_string();
                    Ok(ParsedUriV2::UniversalTag {
                        user_id,
                        app: app_segment.to_string(),
                        resource: Resource::Tag(tag_id.clone()),
                        tag_id,
                    })
                }
                _ => Err(format!(
                    "Unsupported URI format for app '{}': {}",
                    app_segment, uri
                )),
            }
        }
    }
}

impl TryFrom<String> for ParsedUriV2 {
    type Error = String;

    fn try_from(uri: String) -> Result<Self, Self::Error> {
        ParsedUriV2::try_from(uri.as_str())
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use crate::utils::*;

    use super::*;

    const USER_ID: &str = "operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo";

    #[test]
    fn test_empty_bookmark_uri() {
        let uri = bookmark_uri_builder(USER_ID.into(), "".into());
        let parsed_uri = ParsedUri::try_from(uri).unwrap_or_default();
        assert_eq!(
            parsed_uri.resource,
            Resource::Unknown,
            "The provided URI has bookmark_id"
        );
    }

    #[test]
    fn test_some_bookmark_uri() {
        let uri = bookmark_uri_builder(USER_ID.into(), "00".into());
        let parsed_uri = ParsedUri::try_from(uri).unwrap_or_default();
        assert_eq!(
            parsed_uri.resource,
            Resource::Bookmark("00".to_string()),
            "The provided URI has wrong id"
        );
    }

    #[test]
    fn test_user() {
        let uri = user_uri_builder(USER_ID.into());
        let parsed_uri = ParsedUri::try_from(uri).unwrap_or_default();
        assert_eq!(
            parsed_uri.resource,
            Resource::User,
            "The provided URI is not user resource type"
        );
    }

    // Successful cases

    #[test]
    fn test_valid_user_uri() {
        let user_id = PubkyId::try_from(USER_ID).unwrap();

        // A valid user URI ends with profile.json.
        let uri = user_uri_builder(USER_ID.into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid user URI");
        assert_eq!(parsed.user_id, user_id);
        assert_eq!(parsed.resource, Resource::User);

        // Repeat same checks for ParsedUriV2 derived directly from PubkyId
        let parsed_uri_from_pubky_id = user_id.to_uri();
        assert_eq!(*parsed_uri_from_pubky_id.user_id(), user_id);
        assert_eq!(*parsed_uri_from_pubky_id.resource(), Resource::User);
    }

    #[test]
    fn test_valid_last_read_uri() {
        // A valid last_read URI ends with last_read.
        let uri = last_read_uri_builder(USER_ID.into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid last_read URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::LastRead);
    }

    #[test]
    fn test_valid_post_uri() {
        // A valid post URI includes the posts/ segment followed by an identifier.
        let uri = post_uri_builder(USER_ID.into(), "0032SSN7Q4EVG".into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid post URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::Post("0032SSN7Q4EVG".to_string()));
    }

    #[test]
    fn test_valid_follow_uri() {
        // A valid follow URI.
        let uri = follow_uri_builder(USER_ID.into(), USER_ID.into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid follow URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        // Assuming PubkyId::try_from("def456") returns a PubkyId that equals PubkyId::try_from("def456")
        assert_eq!(
            parsed.resource,
            Resource::Follow(PubkyId::try_from(USER_ID).unwrap())
        );
    }

    #[test]
    fn test_valid_bookmark_uri() {
        let bookmark_id = "8Z8CWH8NVYQY39ZEBFGKQWWEKG";
        let uri = bookmark_uri_builder(USER_ID.into(), bookmark_id.into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid bookmark URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::Bookmark(bookmark_id.to_string()));
    }

    #[test]
    fn test_valid_tag_uri() {
        let uri = tag_uri_builder(USER_ID.into(), "8Z8CWH8NVYQY39ZEBFGKQWWEKG".into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid tag URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(
            parsed.resource,
            Resource::Tag("8Z8CWH8NVYQY39ZEBFGKQWWEKG".to_string())
        );
    }

    #[test]
    fn test_valid_file_uri() {
        let uri = file_uri_builder(USER_ID.into(), "file003".into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid file URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::File("file003".to_string()));
    }

    #[test]
    fn test_valid_blob_uri() {
        let uri = blob_uri_builder(USER_ID.into(), "8Z8CWH8NVYQY39ZEBFGKQWWEKG".into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid blob URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(
            parsed.resource,
            Resource::Blob("8Z8CWH8NVYQY39ZEBFGKQWWEKG".to_string())
        );
    }

    #[test]
    fn test_valid_feed_uri() {
        let uri = feed_uri_builder(USER_ID.into(), "8Z8CWH8NVYQY39ZEBFGKQWWEKG".into());
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse valid feed URI");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(
            parsed.resource,
            Resource::Feed("8Z8CWH8NVYQY39ZEBFGKQWWEKG".to_string())
        );
    }

    #[test]
    fn test_no_resource_segments() {
        // When there are no segments beyond the public and app paths,
        // the resource should be Unknown.
        let uri = format!("pubky://{USER_ID}/pub/pubky.app/");
        let parsed =
            ParsedUri::try_from(uri).expect("Failed to parse URI with no resource segments");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::Unknown);
    }

    #[test]
    fn test_unknown_resource() {
        // Unknown resource type yields Resource::Unknown.
        let uri = format!("pubky://{USER_ID}/pub/pubky.app/unknown/xyz");
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse URI with unknown resource");
        assert_eq!(parsed.user_id, PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(parsed.resource, Resource::Unknown);
    }

    // Failure cases

    #[test]
    fn test_invalid_scheme() {
        let uri = format!("http://{USER_ID}/pub/pubky.app/profile.json");
        let result = ParsedUri::try_from(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_public_path() {
        // Change the public path so it doesn't match.
        let uri = format!("pubky://{USER_ID}/invalid/pubky.app/profile.json");
        let result = ParsedUri::try_from(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_app_path() {
        // Change the app path so it doesn't match.
        let uri = format!("pubky://{USER_ID}/pub/other.app/profile.json");
        let result = ParsedUri::try_from(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_host() {
        // URL with missing host.
        let uri = "pubky:///pub/pubky.app/profile.json";
        let result = ParsedUri::try_from(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url() {
        let uri = "not a url";
        let result = ParsedUri::try_from(uri);
        assert!(result.is_err());
    }

    // Reverse conversion tests: ParsedUri::try_to_uri_str should produce the same string as the builder functions

    #[test]
    fn test_user_uri_roundtrip() {
        let original_uri = user_uri_builder(USER_ID.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse user URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "User URI roundtrip failed");
    }

    #[test]
    fn test_last_read_uri_roundtrip() {
        let original_uri = last_read_uri_builder(USER_ID.into());
        let parsed =
            ParsedUri::try_from(original_uri.clone()).expect("Failed to parse last_read URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(
            original_uri, reconstructed_uri,
            "LastRead URI roundtrip failed"
        );
    }

    #[test]
    fn test_post_uri_roundtrip() {
        let post_id = "0032SSN7Q4EVG";
        let original_uri = post_uri_builder(USER_ID.into(), post_id.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse post URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "Post URI roundtrip failed");
    }

    #[test]
    fn test_follow_uri_roundtrip() {
        let original_uri = follow_uri_builder(USER_ID.into(), USER_ID.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse follow URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(
            original_uri, reconstructed_uri,
            "Follow URI roundtrip failed"
        );
    }

    #[test]
    fn test_mute_uri_roundtrip() {
        let original_uri = mute_uri_builder(USER_ID.into(), USER_ID.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse mute URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "Mute URI roundtrip failed");
    }

    #[test]
    fn test_bookmark_uri_roundtrip() {
        let bookmark_id = "8Z8CWH8NVYQY39ZEBFGKQWWEKG";
        let original_uri = bookmark_uri_builder(USER_ID.into(), bookmark_id.into());
        let parsed =
            ParsedUri::try_from(original_uri.clone()).expect("Failed to parse bookmark URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(
            original_uri, reconstructed_uri,
            "Bookmark URI roundtrip failed"
        );
    }

    #[test]
    fn test_tag_uri_roundtrip() {
        let tag_id = "8Z8CWH8NVYQY39ZEBFGKQWWEKG";
        let original_uri = tag_uri_builder(USER_ID.into(), tag_id.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse tag URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "Tag URI roundtrip failed");
    }

    #[test]
    fn test_file_uri_roundtrip() {
        let file_id = "file003";
        let original_uri = file_uri_builder(USER_ID.into(), file_id.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse file URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "File URI roundtrip failed");
    }

    #[test]
    fn test_blob_uri_roundtrip() {
        let blob_id = "8Z8CWH8NVYQY39ZEBFGKQWWEKG";
        let original_uri = blob_uri_builder(USER_ID.into(), blob_id.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse blob URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "Blob URI roundtrip failed");
    }

    #[test]
    fn test_feed_uri_roundtrip() {
        let feed_id = "8Z8CWH8NVYQY39ZEBFGKQWWEKG";
        let original_uri = feed_uri_builder(USER_ID.into(), feed_id.into());
        let parsed = ParsedUri::try_from(original_uri.clone()).expect("Failed to parse feed URI");
        let reconstructed_uri = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed_uri, "Feed URI roundtrip failed");
    }

    #[test]
    fn test_unknown_resource_to_uri_str_fails() {
        let uri = format!("pubky://{USER_ID}/pub/pubky.app/unknown/xyz");
        let parsed = ParsedUri::try_from(uri).expect("Failed to parse URI with unknown resource");
        assert_eq!(parsed.resource, Resource::Unknown);
        let result = parsed.try_to_uri_str();
        assert!(
            result.is_err(),
            "Unknown resource should fail to convert to URI string"
        );
    }

    // =========================================================================
    // ParsedUriV2 tests
    // =========================================================================

    #[test]
    fn test_v2_appspec_post_uri() {
        let uri = post_uri_builder(USER_ID.into(), "0032SSN7Q4EVG".into());
        let parsed = ParsedUriV2::try_from(uri.as_str()).expect("Failed to parse post URI");
        match &parsed {
            ParsedUriV2::AppSpec { user_id, resource } => {
                assert_eq!(user_id, &PubkyId::try_from(USER_ID).unwrap());
                assert_eq!(*resource, Resource::Post("0032SSN7Q4EVG".to_string()));
            }
            _ => panic!("Expected AppSpec variant"),
        }
        assert_eq!(parsed.user_id(), &PubkyId::try_from(USER_ID).unwrap());
        assert_eq!(
            *parsed.resource(),
            Resource::Post("0032SSN7Q4EVG".to_string())
        );
    }

    #[test]
    fn test_v2_appspec_user_uri() {
        let uri = user_uri_builder(USER_ID.into());
        let parsed = ParsedUriV2::try_from(uri.as_str()).expect("Failed to parse user URI");
        assert!(matches!(
            parsed,
            ParsedUriV2::AppSpec {
                resource: Resource::User,
                ..
            }
        ));
    }

    #[test]
    fn test_v2_appspec_roundtrip() {
        let original_uri = tag_uri_builder(USER_ID.into(), "8Z8CWH8NVYQY39ZEBFGKQWWEKG".into());
        let parsed =
            ParsedUriV2::try_from(original_uri.as_str()).expect("Failed to parse tag URI");
        let reconstructed = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed);
    }

    #[test]
    fn test_v2_appspec_unknown_resource_to_uri_fails() {
        let uri = format!("pubky://{USER_ID}/pub/pubky.app/unknown/xyz");
        let parsed =
            ParsedUriV2::try_from(uri.as_str()).expect("Failed to parse URI with unknown resource");
        assert!(matches!(
            parsed,
            ParsedUriV2::AppSpec {
                resource: Resource::Unknown,
                ..
            }
        ));
        assert!(parsed.try_to_uri_str().is_err());
    }

    #[test]
    fn test_v2_universal_tag_uri() {
        let tag_id = "SOMETAGID123";
        let uri = format!("pubky://{USER_ID}/pub/other.app/tags/{tag_id}");
        let parsed =
            ParsedUriV2::try_from(uri.as_str()).expect("Failed to parse universal tag URI");
        match &parsed {
            ParsedUriV2::UniversalTag {
                user_id,
                app,
                resource,
                tag_id: tid,
            } => {
                assert_eq!(user_id, &PubkyId::try_from(USER_ID).unwrap());
                assert_eq!(app, "other.app");
                assert_eq!(*resource, Resource::Tag(tag_id.to_string()));
                assert_eq!(tid, tag_id);
            }
            _ => panic!("Expected UniversalTag variant"),
        }
    }

    #[test]
    fn test_v2_universal_tag_roundtrip() {
        let tag_id = "SOMETAGID123";
        let original_uri = format!("pubky://{USER_ID}/pub/other.app/tags/{tag_id}");
        let parsed = ParsedUriV2::try_from(original_uri.as_str())
            .expect("Failed to parse universal tag URI");
        let reconstructed = parsed
            .try_to_uri_str()
            .expect("Failed to convert to URI string");
        assert_eq!(original_uri, reconstructed);
    }

    #[test]
    fn test_v2_universal_tag_different_apps() {
        for app in &["my-cool-app", "social.xyz", "notes.example.com"] {
            let uri = format!("pubky://{USER_ID}/pub/{app}/tags/TAGID001");
            let parsed = ParsedUriV2::try_from(uri.as_str())
                .unwrap_or_else(|e| panic!("Failed for app '{}': {}", app, e));
            match &parsed {
                ParsedUriV2::UniversalTag {
                    app: parsed_app, ..
                } => {
                    assert_eq!(parsed_app, *app);
                }
                _ => panic!("Expected UniversalTag variant for app '{}'", app),
            }
        }
    }

    #[test]
    fn test_v2_non_tag_foreign_app_fails() {
        // A non-tag resource under a foreign app should fail
        let uri = format!("pubky://{USER_ID}/pub/other.app/posts/POSTID001");
        let result = ParsedUriV2::try_from(uri.as_str());
        assert!(result.is_err());
    }

    #[test]
    fn test_v2_universal_tag_empty_tag_id_fails() {
        let uri = format!("pubky://{USER_ID}/pub/other.app/tags/");
        let result = ParsedUriV2::try_from(uri.as_str());
        assert!(result.is_err());
    }

    #[test]
    fn test_v2_try_from_string() {
        let uri = post_uri_builder(USER_ID.into(), "0032SSN7Q4EVG".into());
        let parsed = ParsedUriV2::try_from(uri).expect("Failed to parse from String");
        assert!(matches!(parsed, ParsedUriV2::AppSpec { .. }));
    }

    #[test]
    fn test_v2_invalid_scheme() {
        let uri = format!("http://{USER_ID}/pub/pubky.app/profile.json");
        let result = ParsedUriV2::try_from(uri.as_str());
        assert!(result.is_err());
    }

    #[test]
    fn test_v2_invalid_url() {
        let result = ParsedUriV2::try_from("not a url");
        assert!(result.is_err());
    }
}
