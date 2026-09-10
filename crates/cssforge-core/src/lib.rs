pub mod discovery;
pub mod engine;
pub mod model;
pub mod output;
pub mod scanner;

pub use discovery::{discover_css_files, is_git_dirty};
pub use engine::{
    analyze_file, analyze_workspace, apply_selected_plans, apply_until_stable,
    extract_style_blocks, unified_diff,
};
pub use model::*;
pub use output::{OutputOptions, WriteResult, write_result};
