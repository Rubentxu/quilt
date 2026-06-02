# Journal Date Format Specification

## Purpose

User-configurable strftime format for journal page display titles. Journal-day integer (YYYYMMDD) is the canonical DB identity; the formatted title is for display only.

## Requirements

### Requirement: Journal Title Uses User-Configured Format

The system SHALL generate journal page display titles using the user's configured strftime format string.

#### Scenario: ISO format default

- GIVEN journal_format = "%Y-%m-%d"
- WHEN a journal page for 2026-05-27 is rendered
- THEN the title displayed is "2026-05-27"

#### Scenario: European format

- GIVEN journal_format = "%d-%m-%Y"
- WHEN a journal page for 2026-05-27 is rendered
- THEN the title displayed is "27-05-2026"

#### Scenario: Full month name format

- GIVEN journal_format = "%B %d, %Y"
- WHEN a journal page for 2026-05-27 is rendered
- THEN the title displayed is "May 27, 2026"

### Requirement: Route Params Stay ISO

The system SHALL use ISO format (YYYY-MM-DD) for journal route parameters to ensure URL stability.

#### Scenario: Navigation uses ISO date

- GIVEN user is on /journal/2026-05-27
- WHEN user navigates to next day
- THEN URL becomes /journal/2026-05-28
- AND title displayed uses user's configured format

### Requirement: Journal Day Lookup By Integer

The system SHALL look up journal pages by their journal_day integer (YYYYMMDD), not by the formatted title string.

#### Scenario: Lookup by journal-day integer

- GIVEN journal page for 2026-05-27 exists with journal_day = 20260527
- WHEN searching for this journal page
- THEN the query uses journal_day = 20260527
- AND format setting does not affect the lookup

### Requirement: Journal Page Auto-Creation

The system SHALL auto-create a journal page when accessing a date that does not have a page yet.

#### Scenario: First access of journal day

- GIVEN no page exists for 2026-05-27
- WHEN user navigates to /journal/2026-05-27
- THEN a new journal page is created
- AND its journal_day = 20260527
