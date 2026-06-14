use crate::ast::QueryAst;

pub(crate) fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| input.contains(p))
}

/// Extract text after one of the given patterns.
pub(crate) fn extract_after_pattern<'a>(input: &'a str, patterns: &[&str]) -> Option<&'a str> {
    for pattern in patterns {
        if let Some(pos) = input.find(pattern) {
            let after = &input[pos + pattern.len()..];
            if !after.is_empty() {
                return Some(after.trim());
            }
        }
    }
    None
}

/// Capitalize the first character of a string.
pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Extract top-N value from patterns like "top 10", "los 5 más", etc.
pub(crate) fn extract_top_n(input: &str) -> Option<usize> {
    let lower = input.to_lowercase();

    // English: "top N", "top N blocks"
    if let Some(idx) = lower.find("top ") {
        let after = &lower[idx + "top ".len()..];
        let n: usize = after
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()?;
        if n > 0 {
            return Some(n);
        }
    }

    // Spanish: "los N más", "los 5 más importantes"
    if let Some(idx) = lower.find("los ") {
        let after = &lower[idx + "los ".len()..];
        let n: usize = after
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()?;
        if n > 0 {
            return Some(n);
        }
    }

    // Default
    None
}

/// Convert QueryAst to a DSL string representation.
pub(crate) fn ast_to_dsl(ast: &QueryAst) -> String {
    match ast {
        QueryAst::Task(markers) => {
            format!("(task {})", markers.join(" "))
        }
        QueryAst::Priority(levels) => {
            format!("(priority {})", levels.join(" "))
        }
        QueryAst::Page(name) => {
            format!("(page \"{}\")", name)
        }
        QueryAst::Tags(tag) => {
            format!("(tag \"{}\")", tag)
        }
        QueryAst::Property { key, op, value, value2 } => {
            let op_str = match op {
                crate::property_op::PropertyOp::Equals => "",
                crate::property_op::PropertyOp::NotEquals => " != ",
                crate::property_op::PropertyOp::GreaterThan => " > ",
                crate::property_op::PropertyOp::LessThan => " < ",
                crate::property_op::PropertyOp::GreaterThanOrEqual => " >= ",
                crate::property_op::PropertyOp::LessThanOrEqual => " <= ",
                crate::property_op::PropertyOp::Contains => " contains ",
                crate::property_op::PropertyOp::Between => " between ",
            };
            if let Some(v2) = value2 {
                format!("(property \"{}\" {} {} {})", key, value, op_str, v2)
            } else if op_str.is_empty() {
                format!("(property \"{}\" {})", key, value)
            } else {
                format!("(property \"{}\" {}{})", key, op_str.trim(), value)
            }
        }
        QueryAst::Between { field, start, end } => {
            format!("(between \"{}\" {} {})", field, start, end)
        }
        QueryAst::And(children) => {
            format!("(and {})", children.iter().map(|c| ast_to_dsl(c)).collect::<Vec<_>>().join(" "))
        }
        QueryAst::Or(children) => {
            format!("(or {})", children.iter().map(|c| ast_to_dsl(c)).collect::<Vec<_>>().join(" "))
        }
        QueryAst::Not(child) => {
            format!("(not {})", ast_to_dsl(child))
        }
        QueryAst::PageRef(name) => {
            format!("(page-ref \"{}\")", name)
        }
        QueryAst::SelfRef => "(self)".to_string(),
        QueryAst::BlockContent(content) => {
            format!("(content \"{}\")", content)
        }
        QueryAst::Sample(n) => {
            format!("(sample {})", n)
        }
        QueryAst::Aggregate { inner, group_by, aggregate_fn: _ } => {
            let inner_dsl = ast_to_dsl(&inner);
            format!("(aggregate {} by \"{}\")", inner_dsl, group_by)
        }
        QueryAst::Stats { property, compute: _ } => {
            format!("(stats \"{}\")", property)
        }
        QueryAst::GroupBy { inner, property } => {
            let inner_dsl = ast_to_dsl(&inner);
            format!("(group-by {} by \"{}\")", inner_dsl, property)
        }
        QueryAst::Analyze { .. } => "(analyze)".to_string(),
        _ => format!("{:?}", ast),
    }
}
