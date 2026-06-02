# Settings API Specification

## Purpose

REST API for reading and writing user settings including journal date format, timezone, and preferred content format.

## Requirements

### Requirement: Get User Settings

The system SHALL provide GET /api/v1/settings returning the current UserSettings as JSON.

#### Scenario: Fetch settings

- GIVEN user settings exist with journal_format = "%Y-%m-%d"
- WHEN GET /api/v1/settings is called
- THEN returns {"timezone": "UTC", "journalFormat": "%Y-%m-%d", "startOfWeek": 1, "preferredFormat": "markdown"}

### Requirement: Update User Settings

The system SHALL provide PUT /api/v1/settings accepting partial UserSettings JSON.

#### Scenario: Update journal format

- GIVEN existing settings with journal_format = "%Y-%m-%d"
- WHEN PUT /api/v1/settings with {"journalFormat": "%d-%m-%Y"} is called
- THEN journal_format is updated to "%d-%m-%d"
- AND other settings remain unchanged

#### Scenario: Invalid timezone rejected

- GIVEN settings with timezone = "UTC"
- WHEN PUT /api/v1/settings with {"timezone": "Invalid/Zone"} is called
- THEN returns 400 Bad Request
- AND settings are not modified

### Requirement: Settings Persisted in SQLite

The system SHALL store settings in the SQLite database using the config table.

#### Scenario: Settings survive restart

- GIVEN user updates journal_format to "%d-%m-%Y"
- WHEN server is restarted
- THEN GET /api/v1/settings returns "%d-%m-%Y"
