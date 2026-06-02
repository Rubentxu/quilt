# Inline Markdown Rendering Specification

## Purpose

Parse and render inline markdown syntax (bold, italic, code, links) in block display mode so users see formatted text instead of raw markdown.

## Requirements

### Requirement: Bold Text Rendering

The system SHALL parse `**text**` syntax and render it as `<strong>` in display mode.

#### Scenario: Bold text in block content

- GIVEN a block with content `"This is **important** text"`
- WHEN the block is in display mode (not editing)
- THEN "important" SHALL render as bold text within the sentence

#### Scenario: Multiple bold segments

- GIVEN content `"**first** and **second**"`
- THEN both "first" and "second" SHALL render as bold

### Requirement: Italic Text Rendering

The system SHALL parse `*text*` syntax and render it as `<em>` in display mode. Single asterisks inside a word (e.g., `foo_bar`) MUST NOT be parsed as italic.

#### Scenario: Italic text

- GIVEN content `"This is *emphasized* text"`
- THEN "emphasized" SHALL render as italic

#### Scenario: Asterisk in identifier not parsed

- GIVEN content `"variable_name has a value"`
- THEN the text SHALL render as plain text without italic formatting

### Requirement: Inline Code Rendering

The system SHALL parse `` `code` `` syntax and render it as `<code>` in display mode.

#### Scenario: Code in sentence

- GIVEN content `"Use the `print()` function"`
- THEN `print()` SHALL render in a monospace code element

### Requirement: Link Rendering

The system SHALL parse `[text](url)` syntax and render it as a clickable `<a href>` in display mode.

#### Scenario: Link in content

- GIVEN content `"Visit [Quilt](https://quilt.dev)"`
- THEN "Quilt" SHALL render as a clickable link pointing to `https://quilt.dev`

#### Scenario: Link without text

- GIVEN content `"`[ ](https://example.com)`"`
- THEN an empty link SHALL render (no crash)

### Requirement: Combined Inline Syntax

The system SHALL handle combinations of bold, italic, code, and links within a single block. Bold/italic/code/link parsing MUST NOT interfere with existing `[[page]]`, `((block))`, `#tag`, or `property:: value` syntax.

#### Scenario: Mixed syntax

- GIVEN content `"Check **bold** and *italic* and `code`"`
- THEN all three inline formats SHALL render correctly in the same block

#### Scenario: Inline does not conflict with page refs

- GIVEN content `"See [[My Page]] for **details**"`
- THEN `[[My Page]]` SHALL render as page ref AND "details" as bold

### Requirement: Parsing Priority

Properties (`key:: value`) SHALL be parsed BEFORE markdown inline syntax. Markdown syntax within property values MUST NOT be rendered.

#### Scenario: Property value not rendered as markdown

- GIVEN content `"status:: **active**"`
- THEN the property value SHALL display as plain "**active**" text within the property badge
