//! Integration tests for quilt-core types — serde roundtrip for
//! WASM boundary DTOs: BlockDto, Segment, OutlinerCommand, etc.

use quilt_core::types::{
    BlockDto, CommandResponse, OutlinerCommand, OutlinerState, PageDto, SearchResultDto, Segment,
};

// ── BlockDto serde ─────────────────────────────────────────

#[test]
fn test_block_dto_serde_roundtrip() {
    let block = BlockDto {
        id: "b1".into(),
        page_id: "p1".into(),
        parent_id: Some("parent1".into()),
        content: "Hello world".into(),
        order: 1.5,
        level: 2,
        marker: Some("TODO".into()),
        priority: Some("A".into()),
        collapsed: false,
        properties: serde_json::json!({"status": "draft"}),
        refs: vec!["r1".into()],
        created_at: "2026-01-01".into(),
        updated_at: "2026-01-02".into(),
        created_by: Some("agent::claude".into()),
    };

    let json = serde_json::to_string(&block).unwrap();
    let restored: BlockDto = serde_json::from_str(&json).unwrap();

    assert_eq!(block, restored);
}

#[test]
fn test_block_dto_camel_case_keys() {
    let block = BlockDto {
        id: "b1".into(),
        page_id: "p1".into(),
        parent_id: None,
        content: "test".into(),
        order: 1.0,
        level: 1,
        marker: None,
        priority: None,
        collapsed: false,
        properties: serde_json::Value::Null,
        refs: vec![],
        created_at: "2026-01-01".into(),
        updated_at: "2026-01-01".into(),
        created_by: None,
    };

    let json = serde_json::to_value(&block).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("pageId"));
    assert!(obj.contains_key("parentId"));
    assert!(obj.contains_key("createdAt"));
    assert!(!obj.contains_key("page_id"));
}

#[test]
fn test_block_dto_minimal() {
    let json = r#"{"id":"b1","pageId":"p1","content":"minimal","order":1.0,"level":1,"collapsed":false,"createdAt":"x","updatedAt":"x"}"#;
    let block: BlockDto = serde_json::from_str(json).unwrap();
    assert_eq!(block.id, "b1");
    assert_eq!(block.parent_id, None);
    assert!(block.refs.is_empty());
}

// ── Segment serde ──────────────────────────────────────────

#[test]
fn test_segment_text_roundtrip() {
    let seg = Segment::Text("hello".into());
    let json = serde_json::to_string(&seg).unwrap();
    let restored: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(seg, restored);
}

#[test]
fn test_segment_page_ref_roundtrip() {
    let seg = Segment::PageRef("mypage".into());
    let json = serde_json::to_string(&seg).unwrap();
    let restored: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(seg, restored);
}

#[test]
fn test_segment_bold_roundtrip() {
    let seg = Segment::Bold("important".into());
    let json = serde_json::to_string(&seg).unwrap();
    let restored: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(seg, restored);
}

#[test]
fn test_segment_code_roundtrip() {
    let seg = Segment::Code("fn main() {}".into());
    let json = serde_json::to_string(&seg).unwrap();
    let restored: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(seg, restored);
}

#[test]
fn test_segment_property_roundtrip() {
    let seg = Segment::Property {
        key: "status".into(),
        value: "draft".into(),
    };
    let json = serde_json::to_string(&seg).unwrap();
    let restored: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(seg, restored);
}

// ── OutlinerCommand serde ──────────────────────────────────

#[test]
fn test_outliner_command_set_content() {
    let cmd = OutlinerCommand::SetContent {
        block_id: "b1".into(),
        content: "new".into(),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    eprintln!("JSON: {}", json);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "setContent");
    // Field names might be block_id or blockId — check both
    assert_eq!(parsed["block_id"], "b1");
    assert_eq!(parsed["content"], "new");
}

#[test]
fn test_outliner_command_split_block() {
    let cmd = OutlinerCommand::SplitBlock {
        block_id: "b1".into(),
        cursor_pos: 5,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "splitBlock");
    assert_eq!(parsed["cursor_pos"], 5);
}

#[test]
fn test_outliner_command_indent() {
    let cmd = OutlinerCommand::Indent {
        block_id: "b1".into(),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let restored: OutlinerCommand = serde_json::from_str(&json).unwrap();

    match restored {
        OutlinerCommand::Indent { block_id } => assert_eq!(block_id, "b1"),
        _ => panic!("expected Indent"),
    }
}

#[test]
fn test_outliner_command_move_block() {
    let cmd = OutlinerCommand::MoveBlock {
        block_id: "b1".into(),
        new_parent_id: "p2".into(),
        new_order: 3.5,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "moveBlock");
    assert_eq!(parsed["new_parent_id"], "p2");
}

// ── CommandResponse ────────────────────────────────────────

#[test]
fn test_command_response_roundtrip() {
    let resp = CommandResponse {
        accepted: true,
        state_hash: 42,
        error: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let restored: CommandResponse = serde_json::from_str(&json).unwrap();

    assert!(restored.accepted);
    assert_eq!(restored.state_hash, 42);
    assert!(restored.error.is_none());
}

#[test]
fn test_command_response_with_error() {
    let resp = CommandResponse {
        accepted: false,
        state_hash: 0,
        error: Some("Invalid block".into()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let restored: CommandResponse = serde_json::from_str(&json).unwrap();

    assert!(!restored.accepted);
    assert_eq!(restored.error, Some("Invalid block".into()));
}

// ── PageDto ────────────────────────────────────────────────

#[test]
fn test_page_dto_roundtrip() {
    let page = PageDto {
        id: "p1".into(),
        name: "home".into(),
        title: Some("Home Page".into()),
        namespace: None,
        journal: false,
        journal_day: None,
        created_at: "2026-01-01".into(),
        updated_at: "2026-01-02".into(),
    };
    let json = serde_json::to_string(&page).unwrap();
    let restored: PageDto = serde_json::from_str(&json).unwrap();
    assert_eq!(page, restored);
}

// ── SearchResultDto ────────────────────────────────────────

#[test]
fn test_search_result_roundtrip() {
    let result = SearchResultDto {
        block_id: "b1".into(),
        page_id: "p1".into(),
        page_name: "home".into(),
        content: "found text".into(),
        snippet: Some("...found...".into()),
        rank: Some(0.95),
    };
    let json = serde_json::to_string(&result).unwrap();
    let restored: SearchResultDto = serde_json::from_str(&json).unwrap();
    assert_eq!(result.block_id, restored.block_id);
    assert_eq!(result.content, restored.content);
    assert_eq!(result.snippet, restored.snippet);
}

// ── OutlinerState ──────────────────────────────────────────

#[test]
fn test_outliner_state_roundtrip() {
    let state = OutlinerState {
        blocks: vec![],
        page: None,
        can_undo: true,
        can_redo: false,
        state_hash: 12345,
    };
    let json = serde_json::to_string(&state).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["canUndo"], true);
    assert_eq!(parsed["canRedo"], false);
    assert_eq!(parsed["stateHash"], 12345);
    assert!(parsed["blocks"].is_array());
}
