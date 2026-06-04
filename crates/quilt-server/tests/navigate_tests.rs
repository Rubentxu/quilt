//! Tests for navigation PageDto conversions.
//!
//! Covers: PageDto::from(Page) for regular pages, journal pages,
//! and pages with null fields.

use chrono::{TimeZone, Utc};
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::value_objects::{BlockFormat, JournalDay};
use quilt_server::handlers::navigate::PageDto;

fn make_page(name: &str, journal_day: Option<JournalDay>) -> Page {
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 15, 0, 0).unwrap();
    Page {
        id: quilt_domain::value_objects::Uuid::new_v4(),
        name: name.to_string(),
        title: Some(format!("Title: {}", name)),
        namespace_id: None,
        journal_day,
        format: BlockFormat::Markdown,
        file_id: None,
        original_name: None,
        journal: journal_day.is_some(),
        created_at: now,
        updated_at: now,
        properties: std::collections::HashMap::new(),
    }
}

#[test]
fn test_page_dto_from_regular_page() {
    let page = make_page("my-page", None);
    let dto = PageDto::from(page.clone());

    assert_eq!(dto.id, page.id.to_string());
    assert_eq!(dto.name, "my-page");
    assert_eq!(dto.title, Some("Title: my-page".to_string()));
    assert!(!dto.journal);
    assert_eq!(dto.journal_day, None);
    assert!(!dto.created_at.is_empty());
}

#[test]
fn test_page_dto_from_journal_page() {
    let day = JournalDay::from_ymd(2026, 6, 2).unwrap();
    let page = make_page("2026-06-02", Some(day));
    let dto = PageDto::from(page.clone());

    assert!(dto.journal);
    assert!(dto.journal_day.is_some());
    assert_eq!(dto.journal_day.unwrap(), day.as_i32() as i64);
}

#[test]
fn test_page_dto_from_page_without_title() {
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 15, 0, 0).unwrap();
    let page = Page {
        id: quilt_domain::value_objects::Uuid::new_v4(),
        name: "no-title".to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        original_name: None,
        journal: false,
        created_at: now,
        updated_at: now,
        properties: std::collections::HashMap::new(),
    };
    let dto = PageDto::from(page);
    assert_eq!(dto.title, None);
    assert!(!dto.journal);
}
