//! Metrics widgets for the cognitive dashboard
//!
//! Provides additional metric visualizations for activity tracking,
//! provenance metrics, and graph health.

use leptos::prelude::*;
use crate::bridge::{BriefingStatsDto, CognitivePulseDto};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricCardSize {
    Small,
    Medium,
    Large,
}

#[component]
pub fn MetricCard(
    label: String,
    value: String,
    icon: String,
    trend: Option<TrendDirection>,
    size: MetricCardSize,
) -> impl IntoView {
    let trend_view = trend.map(|t| {
        view! {
            <span class={format!("metric-trend trend-{}", match t {
                TrendDirection::Up => "up",
                TrendDirection::Down => "down",
                TrendDirection::Neutral => "neutral",
            })}>
                {match t {
                    TrendDirection::Up => "↑",
                    TrendDirection::Down => "↓",
                    TrendDirection::Neutral => "→",
                }}
            </span>
        }
    });

    let size_class = match size {
        MetricCardSize::Small => "metric-card metric-card-small",
        MetricCardSize::Medium => "metric-card metric-card-medium",
        MetricCardSize::Large => "metric-card metric-card-large",
    };

    view! {
        <div class={size_class}>
            <div class="metric-icon">{icon}</div>
            <div class="metric-content">
                <div class="metric-value">{value}</div>
                <div class="metric-label">{label}</div>
            </div>
            {trend_view}
        </div>
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Up,
    Down,
    Neutral,
}

#[component]
pub fn ActivityChart(stats: BriefingStatsDto) -> impl IntoView {
    let max_value = (stats.pages_created_today
        .max(stats.blocks_created_today))
        .max(stats.queries_run_today)
        .max(1);

    let page_height = (stats.pages_created_today as f64 / max_value as f64 * 100.0).round() as usize;
    let block_height = (stats.blocks_created_today as f64 / max_value as f64 * 100.0).round() as usize;
    let query_height = (stats.queries_run_today as f64 / max_value as f64 * 100.0).round() as usize;

    view! {
        <div class="activity-chart">
            <h4 class="chart-title">"Today's Activity"</h4>
            <div class="chart-container">
                <div class="chart-bar-container">
                    <div class="chart-bar" style={format!("height: {}%", page_height)}>
                        <span class="bar-value">{stats.pages_created_today}</span>
                    </div>
                    <span class="bar-label">"Pages"</span>
                </div>
                <div class="chart-bar-container">
                    <div class="chart-bar" style={format!("height: {}%", block_height)}>
                        <span class="bar-value">{stats.blocks_created_today}</span>
                    </div>
                    <span class="bar-label">"Blocks"</span>
                </div>
                <div class="chart-bar-container">
                    <div class="chart-bar" style={format!("height: {}%", query_height)}>
                        <span class="bar-value">{stats.queries_run_today}</span>
                    </div>
                    <span class="bar-label">"Queries"</span>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn CognitiveMetricsPanel(pulse: CognitivePulseDto) -> impl IntoView {
    view! {
        <div class="cognitive-metrics-panel">
            <h4 class="panel-title">"Graph Health"</h4>
            <div class="metrics-grid">
                <MetricCard
                    label="Total Pages".to_string()
                    value={pulse.total_pages.to_string()}
                    icon="📄".to_string()
                    trend={None}
                    size={MetricCardSize::Small}
                />
                <MetricCard
                    label="Total Blocks".to_string()
                    value={pulse.total_blocks.to_string()}
                    icon="📝".to_string()
                    trend={None}
                    size={MetricCardSize::Small}
                />
                <MetricCard
                    label="Clusters".to_string()
                    value={pulse.clusters.to_string()}
                    icon="🔗".to_string()
                    trend={None}
                    size={MetricCardSize::Small}
                />
                <MetricCard
                    label="Frontiers".to_string()
                    value={pulse.frontiers.to_string()}
                    icon="🌐".to_string()
                    trend={None}
                    size={MetricCardSize::Small}
                />
                <MetricCard
                    label="Knowledge Gaps".to_string()
                    value={pulse.gaps.to_string()}
                    icon="❓".to_string()
                    trend={None}
                    size={MetricCardSize::Small}
                />
            </div>
        </div>
    }
}

#[component]
pub fn ProvenanceBar(score: f64) -> impl IntoView {
    let percentage = (score * 100.0).round() as usize;
    let color_class = if score >= 0.7 {
        "provenance-bar-fill provenance-high"
    } else if score >= 0.4 {
        "provenance-bar-fill provenance-medium"
    } else {
        "provenance-bar-fill provenance-low"
    };

    view! {
        <div class="provenance-bar-container">
            <div class="provenance-label">"Provenance Score"</div>
            <div class="provenance-bar-track">
                <div class={color_class} style={format!("width: {}%", percentage)}></div>
            </div>
            <div class="provenance-value">{percentage}%</div>
        </div>
    }
}

#[component]
pub fn StatusBadge(
    label: String,
    status_type: StatusType,
) -> impl IntoView {
    let status_class = match status_type {
        StatusType::Success => "status-badge status-success",
        StatusType::Warning => "status-badge status-warning",
        StatusType::Error => "status-badge status-error",
        StatusType::Info => "status-badge status-info",
    };

    view! {
        <span class={status_class}>
            {label}
        </span>
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusType {
    Success,
    Warning,
    Error,
    Info,
}
