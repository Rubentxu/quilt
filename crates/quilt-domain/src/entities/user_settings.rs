//! UserSettings entity - persisted user preferences
//!
//! These settings are persisted in the database and control
//! how the application behaves for a specific user.

use crate::errors::DomainError;
use crate::services::TimezoneService;
use crate::value_objects::BlockFormat;

/// User settings for personalization.
///
/// These settings are persisted in the database and control
/// how the application behaves for a specific user.
#[derive(Debug, Clone, PartialEq)]
pub struct UserSettings {
    /// User's timezone (IANA format, e.g., "America/Mexico_City")
    pub timezone: String,

    /// Date format for journal pages (strftime format, e.g., "%Y-%m-%d")
    pub journal_format: String,

    /// First day of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    pub start_of_week: u8,

    /// Preferred content format for new pages/blocks
    pub preferred_format: BlockFormat,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            journal_format: "%Y-%m-%d".to_string(),
            start_of_week: 1, // Monday
            preferred_format: BlockFormat::Markdown,
        }
    }
}

impl UserSettings {
    /// Validate the settings.
    ///
    /// # Errors
    /// Returns error if timezone is invalid or start_of_week is out of range.
    pub fn validate(&self) -> Result<(), DomainError> {
        // Validate timezone
        self.timezone.parse::<chrono_tz::Tz>()
            .map_err(|_| DomainError::InvalidTimezone(self.timezone.clone()))?;

        // Validate start_of_week
        if self.start_of_week > 6 {
            return Err(DomainError::InvalidConfiguration(
                format!("start_of_week must be 0-6, got {}", self.start_of_week)
            ));
        }

        // Validate journal_format (basic check - should be parseable)
        if self.journal_format.is_empty() {
            return Err(DomainError::InvalidConfiguration(
                "journal_format cannot be empty".to_string()
            ));
        }

        Ok(())
    }

    /// Get the timezone as a TimezoneService.
    pub fn to_timezone_service(&self) -> Result<TimezoneService, DomainError> {
        TimezoneService::from_tz_string(&self.timezone)
    }

    /// List common timezones for UI dropdown.
    ///
    /// Returns a list of (timezone_id, display_name) pairs.
    pub fn common_timezones() -> Vec<(&'static str, &'static str)> {
        vec![
            ("UTC", "UTC (Coordinated Universal Time)"),
            ("America/Mexico_City", "Mexico City (UTC-6)"),
            ("America/New_York", "New York (UTC-5)"),
            ("America/Chicago", "Chicago (UTC-6)"),
            ("America/Denver", "Denver (UTC-7)"),
            ("America/Los_Angeles", "Los Angeles (UTC-8)"),
            ("America/Toronto", "Toronto (UTC-5)"),
            ("America/Sao_Paulo", "São Paulo (UTC-3)"),
            ("Europe/London", "London (UTC+0)"),
            ("Europe/Paris", "Paris (UTC+1)"),
            ("Europe/Berlin", "Berlin (UTC+1)"),
            ("Europe/Madrid", "Madrid (UTC+1)"),
            ("Europe/Rome", "Rome (UTC+1)"),
            ("Europe/Amsterdam", "Amsterdam (UTC+1)"),
            ("Europe/Moscow", "Moscow (UTC+3)"),
            ("Asia/Dubai", "Dubai (UTC+4)"),
            ("Asia/Kolkata", "Mumbai/Delhi (UTC+5:30)"),
            ("Asia/Bangkok", "Bangkok (UTC+7)"),
            ("Asia/Shanghai", "Shanghai (UTC+8)"),
            ("Asia/Hong_Kong", "Hong Kong (UTC+8)"),
            ("Asia/Singapore", "Singapore (UTC+8)"),
            ("Asia/Tokyo", "Tokyo (UTC+9)"),
            ("Asia/Seoul", "Seoul (UTC+9)"),
            ("Australia/Sydney", "Sydney (UTC+10)"),
            ("Australia/Melbourne", "Melbourne (UTC+10)"),
            ("Pacific/Auckland", "Auckland (UTC+12)"),
        ]
    }

    /// Get common date formats for journal pages.
    pub fn common_date_formats() -> Vec<(&'static str, &'static str)> {
        vec![
            ("%Y-%m-%d", "2026-05-14 (ISO 8601)"),
            ("%d/%m/%Y", "14/05/2026 (Day/Month/Year)"),
            ("%m/%d/%Y", "05/14/2026 (Month/Day/Year)"),
            ("%B %d, %Y", "May 14, 2026 (Full month name)"),
            ("%b %d, %Y", "May 14, 2026 (Short month name)"),
            ("%Y/%m/%d", "2026/05/14 (Year/Month/Day)"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = UserSettings::default();
        assert_eq!(settings.timezone, "UTC");
        assert_eq!(settings.journal_format, "%Y-%m-%d");
        assert_eq!(settings.start_of_week, 1);
        assert_eq!(settings.preferred_format, BlockFormat::Markdown);
    }

    #[test]
    fn test_validate_valid_settings() {
        let settings = UserSettings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_timezone() {
        let settings = UserSettings {
            timezone: "Invalid/Zone".to_string(),
            ..Default::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidTimezone(tz) => assert_eq!(tz, "Invalid/Zone"),
            _ => panic!("Expected InvalidTimezone error"),
        }
    }

    #[test]
    fn test_validate_invalid_start_of_week() {
        let settings = UserSettings {
            start_of_week: 7,
            ..Default::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidConfiguration(msg) => {
                assert!(msg.contains("start_of_week"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }

    #[test]
    fn test_validate_empty_journal_format() {
        let settings = UserSettings {
            journal_format: "".to_string(),
            ..Default::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidConfiguration(msg) => {
                assert!(msg.contains("journal_format"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }

    #[test]
    fn test_to_timezone_service() {
        let settings = UserSettings::default();
        let tz = settings.to_timezone_service().unwrap();
        assert_eq!(tz.timezone_id(), "UTC");
    }

    #[test]
    fn test_common_timezones() {
        let timezones = UserSettings::common_timezones();
        assert!(!timezones.is_empty());
        // Check that UTC is in the list
        assert!(timezones.iter().any(|(id, _)| *id == "UTC"));
        // Check format of first entry
        assert!(timezones[0].1.contains("UTC"));
    }
}