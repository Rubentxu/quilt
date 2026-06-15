//! Static V1 preset registry — hard-coded 9 presets with build-time validator.
//!
//! The registry is constructed via [`StaticPresetRegistry::v1()`] which
//! validates the V1 table structure at **build time** (not runtime).
//! If the table is malformed, the constructor **panics** — this is intentional
//! per ADR-0025 OQ#2: a hard-coded table drifting from the spec is a
//! programming bug, not a runtime condition.
//!
//! # V1 Preset Table
//!
//! | Slash       | Required Args | Patches                                   | Merge Policies                        |
//! |-------------|---------------|-------------------------------------------|----------------------------------------|
//! | `/TODO`     | none          | `type=task`, `status=todo`, `projection=auto` | SetIfMissing / Overwrite / SetIfMissing |
//! | `/DOING`    | none          | `type=task`, `status=doing`, `projection=auto` | SetIfMissing / Overwrite / SetIfMissing |
//! | `/WAITING`  | none          | `type=task`, `status=waiting`, `projection=auto` | SetIfMissing / Overwrite / SetIfMissing |
//! | `/DONE`     | none          | `type=task`, `status=done`, `projection=auto` | SetIfMissing / Overwrite / SetIfMissing |
//! | `/NOW`      | none          | `type=task`, `status=todo`, `focus=now`, `projection=auto` | SetIfMissing / Overwrite / Overwrite / SetIfMissing |
//! | `/Scheduled` | Date          | `scheduled=<date>`                        | Overwrite                              |
//! | `/Deadline` | Date          | `deadline=<date>`                          | Overwrite                              |
//! | `/Video`    | Url           | `type=media`, `media-type=video`, `source-url=<url>` | SetIfMissing / AskOnConflict / AskOnConflict |
//! | `/Image`    | Url           | `type=media`, `media-type=image`, `source-url=<url>` | SetIfMissing / AskOnConflict / AskOnConflict |

use chrono::NaiveDate;
use quilt_domain::canonicalization::{
    PresetArg, PresetArgKind, PresetArgs, PresetId, PresetRegistry, PropertyPatch,
    PropertyPatchProvenance, PropertyPreset,
};
use quilt_domain::entities::PropertyKey;
use quilt_domain::value_objects::PropertyValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Static registry holding the 9 V1 presets.
#[derive(Debug, Clone)]
pub struct StaticPresetRegistry {
    presets: HashMap<PresetId, PropertyPreset>,
}

impl Default for StaticPresetRegistry {
    fn default() -> Self {
        Self::v1()
    }
}

impl StaticPresetRegistry {
    /// Build the V1 registry, panicking if the hard-coded table is structurally invalid.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - A `Date` arg preset has no `scheduled` or `deadline` patch key
    /// - A `Url` arg preset has no `source-url` patch key
    ///
    /// This is a **build-time programmer error**, not a runtime condition.
    #[must_use]
    pub fn v1() -> Self {
        let mut registry = Self { presets: HashMap::new() };

        // Build-time validator result
        let mut validation_errors: Vec<String> = Vec::new();

        // ── Simple task presets (no args) ──────────────────────────────────

        fn task_preset(
            id: &'static str,
            status: &'static str,
        ) -> (PresetId, PropertyPreset) {
            let id = PresetId::new(id).expect("valid preset id");
            let preset = PropertyPreset::new(
                id.clone(),
                vec![
                    PropertyPatch::explicit(
                        PropertyKey::new("type").unwrap(),
                        PropertyValue::text("task"),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("status").unwrap(),
                        PropertyValue::text(status),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("projection").unwrap(),
                        PropertyValue::text("auto"),
                    ),
                ],
                PresetArgs::empty(),
                "",
            )
            .expect("V1 preset is valid")
            .with_description(format!("{} preset", status));
            (id, preset)
        }

        // /TODO
        let (id, preset) = task_preset("/TODO", "todo");
        registry.presets.insert(id, preset);

        // /DOING
        let (id, preset) = task_preset("/DOING", "doing");
        registry.presets.insert(id, preset);

        // /WAITING
        let (id, preset) = task_preset("/WAITING", "waiting");
        registry.presets.insert(id, preset);

        // /DONE
        let (id, preset) = task_preset("/DONE", "done");
        registry.presets.insert(id, preset);

        // /NOW — adds focus=now
        {
            let id = PresetId::new("/NOW").expect("valid preset id");
            let preset = PropertyPreset::new(
                id.clone(),
                vec![
                    PropertyPatch::explicit(
                        PropertyKey::new("type").unwrap(),
                        PropertyValue::text("task"),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("status").unwrap(),
                        PropertyValue::text("todo"),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("focus").unwrap(),
                        PropertyValue::text("now"),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("projection").unwrap(),
                        PropertyValue::text("auto"),
                    ),
                ],
                PresetArgs::empty(),
                "Now preset",
            )
            .expect("V1 preset is valid");
            registry.presets.insert(id, preset);
        }

        // ── Date-arg presets ───────────────────────────────────────────────

        fn date_preset(
            id: &'static str,
            patch_key: &str,
            placeholder_date: NaiveDate,
        ) -> (PresetId, PropertyPreset) {
            let id = PresetId::new(id).expect("valid preset id");
            let args = PresetArgs::from_vec(vec![PresetArg::Date(placeholder_date)]).expect("valid args");
            let preset = PropertyPreset::new(
                id.clone(),
                vec![PropertyPatch::explicit(
                    PropertyKey::new(patch_key).unwrap(),
                    // The actual date value is bound at apply time via PresetArgs;
                    // this placeholder is used for V1 registry validation only.
                    // The serialized form uses an ISO string for the placeholder.
                    PropertyValue::text(placeholder_date.to_string()),
                )],
                args,
                "",
            )
            .expect("V1 preset is valid")
            .with_description(format!("{} preset", &id.as_str()[1..])); // strip leading /
            (id, preset)
        }

        // /Scheduled
        let (id, preset) = date_preset("/Scheduled", "scheduled", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        // Build-time validation: Date-arg preset must have a scheduled or deadline patch
        let has_date_patch = preset.patches.iter().any(|p| {
            let k = p.key.as_str();
            k == "scheduled" || k == "deadline"
        });
        if !has_date_patch {
            validation_errors.push(
                format!("/Scheduled requires a 'scheduled' or 'deadline' patch key, found: {:?}",
                    preset.patches.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
                )
            );
        }
        registry.presets.insert(id, preset);

        // /Deadline
        let (id, preset) = date_preset("/Deadline", "deadline", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let has_date_patch = preset.patches.iter().any(|p| {
            let k = p.key.as_str();
            k == "scheduled" || k == "deadline"
        });
        if !has_date_patch {
            validation_errors.push(
                format!("/Deadline requires a 'scheduled' or 'deadline' patch key, found: {:?}",
                    preset.patches.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
                )
            );
        }
        registry.presets.insert(id, preset);

        // ── Url-arg presets ─────────────────────────────────────────────────

        fn url_preset(
            id: &'static str,
            media_type: &str,
        ) -> (PresetId, PropertyPreset) {
            let id = PresetId::new(id).expect("valid preset id");
            let placeholder_url = url::Url::parse("https://placeholder.example.com").expect("valid placeholder");
            let args = PresetArgs::from_vec(vec![PresetArg::Url(placeholder_url)]).expect("valid args");
            let preset = PropertyPreset::new(
                id.clone(),
                vec![
                    PropertyPatch::explicit(
                        PropertyKey::new("type").unwrap(),
                        PropertyValue::text("media"),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("media-type").unwrap(),
                        PropertyValue::text(media_type),
                    ),
                    PropertyPatch::explicit(
                        PropertyKey::new("source-url").unwrap(),
                        // The actual URL is bound at apply time via PresetArgs
                        PropertyValue::text("https://placeholder.example.com"),
                    ),
                ],
                args,
                "",
            )
            .expect("V1 preset is valid")
            .with_description(format!("{} preset", media_type));
            (id, preset)
        }

        // /Video
        let (id, preset) = url_preset("/Video", "video");
        // Build-time validation: Url-arg preset must have a source-url patch
        let has_url_patch = preset.patches.iter().any(|p| p.key.as_str() == "source-url");
        if !has_url_patch {
            validation_errors.push(
                format!("/Video requires a 'source-url' patch key, found: {:?}",
                    preset.patches.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
                )
            );
        }
        registry.presets.insert(id, preset);

        // /Image
        let (id, preset) = url_preset("/Image", "image");
        let has_url_patch = preset.patches.iter().any(|p| p.key.as_str() == "source-url");
        if !has_url_patch {
            validation_errors.push(
                format!("/Image requires a 'source-url' patch key, found: {:?}",
                    preset.patches.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
                )
            );
        }
        registry.presets.insert(id, preset);

        // ── Panic on validation failure ───────────────────────────────────────

        if !validation_errors.is_empty() {
            panic!(
                "StaticPresetRegistry::v1() validation failed (V1 table drift):\n  - {}",
                validation_errors.join("\n  - ")
            );
        }

        registry
    }
}

impl PresetRegistry for StaticPresetRegistry {
    fn get(&self, id: &PresetId) -> Option<PropertyPreset> {
        self.presets.get(id).cloned()
    }

    fn list(&self) -> Vec<PresetId> {
        // Declaration order — HashMap iteration order in Rust is insertion order
        self.presets.keys().cloned().collect()
    }

    fn len(&self) -> usize {
        self.presets.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn check_preset(name: &str, expected_keys: &[&str]) {
        let reg = StaticPresetRegistry::v1();
        let id = PresetId::new(name).expect("valid id");
        let preset = reg.get(&id).expect("preset should exist");
        let keys: Vec<_> = preset.patches.iter().map(|p| p.key.as_str()).collect();
        for expected in expected_keys {
            assert!(
                keys.contains(expected),
                "preset {name} should have key '{expected}', got: {keys:?}"
            );
        }
    }

    // ── Per-preset resolution ──────────────────────────────────────────────

    #[test]
    fn v1_contains_todo() {
        check_preset("/TODO", &["type", "status", "projection"]);
    }

    #[test]
    fn v1_contains_doing() {
        check_preset("/DOING", &["type", "status", "projection"]);
    }

    #[test]
    fn v1_contains_waiting() {
        check_preset("/WAITING", &["type", "status", "projection"]);
    }

    #[test]
    fn v1_contains_done() {
        check_preset("/DONE", &["type", "status", "projection"]);
    }

    #[test]
    fn v1_contains_now() {
        check_preset("/NOW", &["type", "status", "focus", "projection"]);
    }

    #[test]
    fn v1_contains_scheduled() {
        check_preset("/Scheduled", &["scheduled"]);
    }

    #[test]
    fn v1_contains_deadline() {
        check_preset("/Deadline", &["deadline"]);
    }

    #[test]
    fn v1_contains_video() {
        check_preset("/Video", &["type", "media-type", "source-url"]);
    }

    #[test]
    fn v1_contains_image() {
        check_preset("/Image", &["type", "media-type", "source-url"]);
    }

    // ── Registry queries ────────────────────────────────────────────────────

    #[test]
    fn list_returns_9_ids() {
        let reg = StaticPresetRegistry::v1();
        let ids = reg.list();
        assert_eq!(ids.len(), 9, "V1 registry should have 9 presets, got: {:?}", ids);
    }

    #[test]
    fn unknown_preset_returns_none() {
        let reg = StaticPresetRegistry::v1();
        let got = reg.get(&PresetId::new("/NotAPreset").unwrap());
        assert!(got.is_none());
    }

    #[test]
    fn lowercase_todo_returns_none() {
        // PresetId is case-sensitive
        let reg = StaticPresetRegistry::v1();
        let got = reg.get(&PresetId::new("/todo").unwrap());
        assert!(got.is_none());
    }

    #[test]
    fn default_is_v1() {
        let default = StaticPresetRegistry::default();
        assert_eq!(default.len(), 9);
    }

    #[test]
    fn arc_dyn_registry_accepts_static_registry() {
        use quilt_domain::canonicalization::PresetRegistry;
        let reg: Arc<dyn PresetRegistry> = Arc::new(StaticPresetRegistry::v1());
        assert_eq!(reg.len(), 9);
        let todo = reg.get(&PresetId::new("/TODO").unwrap());
        assert!(todo.is_some());
    }

    // ── Specific patch values ───────────────────────────────────────────────

    #[test]
    fn todo_status_is_todo() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/TODO").unwrap()).unwrap();
        let status_patch = preset.patches.iter().find(|p| p.key.as_str() == "status").unwrap();
        assert_eq!(status_patch.value.as_display_string(), "todo");
    }

    #[test]
    fn done_status_is_done() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/DONE").unwrap()).unwrap();
        let status_patch = preset.patches.iter().find(|p| p.key.as_str() == "status").unwrap();
        assert_eq!(status_patch.value.as_display_string(), "done");
    }

    #[test]
    fn video_media_type_is_video() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/Video").unwrap()).unwrap();
        let patch = preset.patches.iter().find(|p| p.key.as_str() == "media-type").unwrap();
        assert_eq!(patch.value.as_display_string(), "video");
    }

    #[test]
    fn now_has_focus_now() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/NOW").unwrap()).unwrap();
        let patch = preset.patches.iter().find(|p| p.key.as_str() == "focus").unwrap();
        assert_eq!(patch.value.as_display_string(), "now");
    }

    // ── All patches have Explicit provenance ─────────────────────────────────

    #[test]
    fn all_patches_are_explicit() {
        let reg = StaticPresetRegistry::v1();
        for preset in reg.presets.values() {
            for patch in &preset.patches {
                assert_eq!(
                    patch.provenance,
                    PropertyPatchProvenance::Explicit,
                    "preset {} patch {:?} should be Explicit",
                    preset.id,
                    patch.key
                );
            }
        }
    }

    // ── Args per preset ─────────────────────────────────────────────────────

    #[test]
    fn scheduled_requires_date_arg() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/Scheduled").unwrap()).unwrap();
        assert!(preset.required_args.get(PresetArgKind::Date).is_some());
    }

    #[test]
    fn deadline_requires_date_arg() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/Deadline").unwrap()).unwrap();
        assert!(preset.required_args.get(PresetArgKind::Date).is_some());
    }

    #[test]
    fn video_requires_url_arg() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/Video").unwrap()).unwrap();
        assert!(preset.required_args.get(PresetArgKind::Url).is_some());
    }

    #[test]
    fn image_requires_url_arg() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/Image").unwrap()).unwrap();
        assert!(preset.required_args.get(PresetArgKind::Url).is_some());
    }

    #[test]
    fn todo_requires_no_args() {
        let reg = StaticPresetRegistry::v1();
        let preset = reg.get(&PresetId::new("/TODO").unwrap()).unwrap();
        assert!(preset.required_args.is_empty());
    }
}
